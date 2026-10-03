use axum::extract::{OriginalUri, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::accounts::keys::{self, KeyError, KeySummary};
use crate::accounts::sessions;
use crate::accounts::users::{self, Identity, Profile};

use crate::auth::Keys;
use crate::constants::{API_KEY_HEADER, NO_STORE_CACHE_CONTROL};
use crate::error::{AppError, Resource};
use crate::extract::SafePath;
use crate::state::AppState;

fn is_internal(keys: &Keys, presented: Option<&str>) -> bool {
    keys.is_internal(presented)
}

async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let presented = request
        .headers()
        .get(API_KEY_HEADER)
        .and_then(|value| value.to_str().ok());
    if !is_internal(&state.keys, presented) {
        let path = request.extensions().get::<OriginalUri>().map_or_else(
            || request.uri().path().to_string(),
            |uri| uri.path().to_string(),
        );
        return AppError::Unauthorized.render_at(&path);
    }
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(NO_STORE_CACHE_CONTROL),
    );
    response
}

async fn upsert_user(
    State(state): State<AppState>,
    Json(identity): Json<Identity>,
) -> Result<Json<Profile>, AppError> {
    match users::upsert(&state.accounts, &identity).await {
        Ok(profile) => Ok(Json(profile)),
        Err(users::UpsertError::EmailTaken) => Err(AppError::EmailTaken),
        Err(users::UpsertError::Database(e)) => Err(e),
    }
}

#[derive(Deserialize)]
struct UserRef {
    user_id: i64,
}

#[derive(Serialize)]
struct CreatedSession {
    id: String,
}

async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<UserRef>,
) -> Result<Json<CreatedSession>, AppError> {
    let id = sessions::create(&state.accounts, body.user_id).await?;
    Ok(Json(CreatedSession {
        id: sessions::encode_id(&id),
    }))
}

async fn resolve_session(
    State(state): State<AppState>,
    SafePath(encoded): SafePath<String>,
) -> Result<Json<Profile>, AppError> {
    let Some(id) = sessions::decode_id(&encoded) else {
        return Err(AppError::Unauthorized);
    };
    sessions::resolve(&state.accounts, &id)
        .await?
        .map(Json)
        .ok_or(AppError::Unauthorized)
}

async fn delete_session(
    State(state): State<AppState>,
    SafePath(encoded): SafePath<String>,
) -> Result<StatusCode, AppError> {
    if let Some(id) = sessions::decode_id(&encoded) {
        sessions::delete(&state.accounts, &id).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_all_sessions(
    State(state): State<AppState>,
    Json(body): Json<UserRef>,
) -> Result<StatusCode, AppError> {
    sessions::delete_all_for(&state.accounts, body.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct CreateKey {
    user_id: i64,
    label: Option<String>,
}

#[derive(Serialize)]
struct CreatedKey {
    value: String,
    prefix: String,
}

#[derive(Deserialize)]
struct RevokeKey {
    user_id: i64,
    key_id: i64,
}

#[derive(Serialize)]
struct AccountView {
    plan: String,
    requests: i32,
    burst: i32,
    used: i64,
    keys: Vec<KeySummary>,
}

async fn list_keys(
    State(state): State<AppState>,
    SafePath(user_id): SafePath<i64>,
) -> Result<Json<AccountView>, AppError> {
    let plan = keys::plan_for(&state.accounts, user_id)
        .await?
        .ok_or(AppError::NotFound(Resource::Account))?;
    let keys = keys::list_for(&state.accounts, user_id).await?;
    Ok(Json(AccountView {
        plan: plan.plan,
        requests: plan.requests,
        burst: plan.burst,
        used: plan.used,
        keys,
    }))
}

async fn create_key(
    State(state): State<AppState>,
    Json(body): Json<CreateKey>,
) -> Result<Json<CreatedKey>, AppError> {
    match keys::create_for(&state.accounts, body.user_id, body.label.as_deref()).await {
        Ok(key) => Ok(Json(CreatedKey {
            value: key.value,
            prefix: key.prefix,
        })),
        Err(KeyError::TooMany) => Err(AppError::TooManyKeys),
        Err(KeyError::NoSuchUser) => Err(AppError::NotFound(Resource::Account)),
        Err(KeyError::Database(e)) => Err(e),
    }
}

async fn revoke_key(
    State(state): State<AppState>,
    Json(body): Json<RevokeKey>,
) -> Result<StatusCode, AppError> {
    match keys::revoke(&state.accounts, body.user_id, body.key_id).await? {
        0 => Err(AppError::NotFound(Resource::ApiKey)),
        _ => Ok(StatusCode::NO_CONTENT),
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", post(upsert_user))
        .route("/users/{id}/keys", get(list_keys))
        .route("/keys", post(create_key))
        .route("/keys/revoke", post(revoke_key))
        .route(
            "/sessions",
            post(create_session).delete(delete_all_sessions),
        )
        .route(
            "/sessions/{id}",
            get(resolve_session).delete(delete_session),
        )
}

pub fn wrap(router: Router<AppState>, state: AppState) -> Router<AppState> {
    router.layer(from_fn_with_state(state, guard))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> Keys {
        Keys::new(Some("internal".into()), Some("full".into()))
    }

    #[test]
    fn the_internal_key_is_accepted() {
        assert!(is_internal(&keys(), Some("internal")));
    }

    #[test]
    fn the_full_key_is_refused() {
        assert!(!is_internal(&keys(), Some("full")));
    }

    #[test]
    fn everything_else_is_refused() {
        assert!(!is_internal(&keys(), Some("qaf_some_users_key")));
        assert!(!is_internal(&keys(), Some("")));
        assert!(!is_internal(&keys(), None));
    }
}
