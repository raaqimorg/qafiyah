use axum::extract::{OriginalUri, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::accounts::keys::{self, KeyError, KeyRecord};
use crate::accounts::sessions;
use crate::accounts::users::{self, Identity, Profile};

use crate::auth::Keys;
use crate::constants::{API_KEY_HEADER, NO_STORE_CACHE_CONTROL};
use crate::error::{AppError, Resource};
use crate::extract::SafePath;
use crate::state::AppState;

#[derive(Deserialize)]
struct IdentityBody {
    provider: String,
    provider_uid: String,
    email: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

impl From<IdentityBody> for Identity {
    fn from(body: IdentityBody) -> Self {
        Identity {
            provider: body.provider,
            provider_uid: body.provider_uid,
            email: body.email,
            display_name: body.display_name,
            avatar_url: body.avatar_url,
        }
    }
}

#[derive(Serialize)]
struct ProfileView {
    id: i64,
    email: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

impl From<Profile> for ProfileView {
    fn from(profile: Profile) -> Self {
        ProfileView {
            id: profile.id,
            email: profile.email,
            display_name: profile.display_name,
            avatar_url: profile.avatar_url,
        }
    }
}

#[derive(Serialize)]
struct KeySummary {
    id: i64,
    prefix: String,
    label: Option<String>,
    created_at: String,
    last_used_at: Option<String>,
    requests_this_hour: i64,
}

fn instant(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

impl From<KeyRecord> for KeySummary {
    fn from(key: KeyRecord) -> Self {
        KeySummary {
            id: key.id,
            prefix: key.prefix,
            label: key.label,
            created_at: instant(key.created_at),
            last_used_at: key.last_used_at.map(instant),
            requests_this_hour: key.requests_this_hour,
        }
    }
}

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
    Json(identity): Json<IdentityBody>,
) -> Result<Json<ProfileView>, AppError> {
    match users::upsert(state.users.as_ref(), &identity.into()).await {
        Ok(profile) => Ok(Json(profile.into())),
        Err(users::UpsertError::EmailTaken) => Err(AppError::EmailTaken),
        Err(users::UpsertError::Store(e)) => Err(e.into()),
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
    let id = sessions::create(state.sessions.as_ref(), body.user_id).await?;
    Ok(Json(CreatedSession {
        id: sessions::encode_id(&id),
    }))
}

async fn resolve_session(
    State(state): State<AppState>,
    SafePath(encoded): SafePath<String>,
) -> Result<Json<ProfileView>, AppError> {
    let Some(id) = sessions::decode_id(&encoded) else {
        return Err(AppError::Unauthorized);
    };
    sessions::resolve(state.sessions.as_ref(), &id)
        .await?
        .map(|profile| Json(profile.into()))
        .ok_or(AppError::Unauthorized)
}

async fn delete_session(
    State(state): State<AppState>,
    SafePath(encoded): SafePath<String>,
) -> Result<StatusCode, AppError> {
    if let Some(id) = sessions::decode_id(&encoded) {
        sessions::delete(state.sessions.as_ref(), &id).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_all_sessions(
    State(state): State<AppState>,
    Json(body): Json<UserRef>,
) -> Result<StatusCode, AppError> {
    state.sessions.delete_all_for(body.user_id).await?;
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
    let plan = state
        .api_keys
        .plan_for(user_id)
        .await?
        .ok_or(AppError::NotFound(Resource::Account))?;
    let keys = state
        .api_keys
        .active_for(user_id)
        .await?
        .into_iter()
        .map(KeySummary::from)
        .collect();
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
    match keys::create_for(state.api_keys.as_ref(), body.user_id, body.label.as_deref()).await {
        Ok(key) => Ok(Json(CreatedKey {
            value: key.value,
            prefix: key.prefix,
        })),
        Err(KeyError::TooMany) => Err(AppError::TooManyKeys),
        Err(KeyError::NoSuchUser) => Err(AppError::NotFound(Resource::Account)),
        Err(KeyError::Store(e)) => Err(e.into()),
    }
}

async fn revoke_key(
    State(state): State<AppState>,
    Json(body): Json<RevokeKey>,
) -> Result<StatusCode, AppError> {
    if state.api_keys.revoke(body.user_id, body.key_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(Resource::ApiKey))
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

    #[test]
    fn a_key_is_listed_with_utc_timestamps_to_the_second() {
        let created = DateTime::from_timestamp(1_791_116_658, 123_000_000).expect("an instant");
        let listed = serde_json::to_value(KeySummary::from(KeyRecord {
            id: 7,
            prefix: "qaf_abcd1234".into(),
            label: None,
            created_at: created,
            last_used_at: None,
            requests_this_hour: 3,
        }))
        .expect("serializable");
        assert_eq!(listed["created_at"], "2026-10-04T12:24:18Z");
        assert!(listed["last_used_at"].is_null());
        assert_eq!(listed["requests_this_hour"], 3);
    }
}
