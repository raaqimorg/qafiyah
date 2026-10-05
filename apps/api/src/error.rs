use std::sync::Arc;

use axum::extract::Request;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::constants::{
    NO_STORE_CACHE_CONTROL, PROD_SITE_URL, RATE_LIMIT_LIMIT_HEADER, RATE_LIMIT_REMAINING_HEADER,
    RATE_LIMIT_RESET_HEADER, UNAVAILABLE_RETRY_AFTER_SECONDS,
};
use crate::domain::StoreError;
use crate::domain::poems::PoemError;
use crate::sentry;

const PROBLEM_JSON: &str = "application/problem+json";

#[derive(Clone, Copy, Debug)]
pub enum Resource {
    Meter,
    Rhyme,
    Theme,
    Era,
    Collection,
    PoemType,
    Poem,
    Poet,
    ApiKey,
    Account,
}

impl Resource {
    fn not_found_detail(self) -> &'static str {
        match self {
            Resource::Meter => "Meter not found",
            Resource::Rhyme => "Rhyme not found",
            Resource::Theme => "Theme not found",
            Resource::Era => "Era not found",
            Resource::Collection => "Collection not found",
            Resource::PoemType => "Poem type not found",
            Resource::Poem => "Poem not found",
            Resource::Poet => "Poet not found",
            Resource::ApiKey => "API key not found",
            Resource::Account => "Account not found",
        }
    }
}

#[derive(Clone, Debug)]
pub struct RouteProblem {
    status: StatusCode,
    code: &'static str,
    title: &'static str,
    detail: String,
}

impl RouteProblem {
    pub fn no_route(method: &Method, path: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "NOT_FOUND",
            title: "Resource not found",
            detail: format!("No route matches {method} {path}"),
        }
    }

    pub fn method_not_allowed(method: &Method, path: &str) -> Self {
        Self {
            status: StatusCode::METHOD_NOT_ALLOWED,
            code: "METHOD_NOT_ALLOWED",
            title: "Method not allowed",
            detail: format!("Method {method} is not allowed for {path}"),
        }
    }

    pub fn bad_request(detail: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "BAD_REQUEST",
            title: "Bad request",
            detail: detail.to_string(),
        }
    }

    pub fn internal(detail: &str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL_SERVER_ERROR",
            title: "Internal server error",
            detail: detail.to_string(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("{0:?} not found")]
    NotFound(Resource),
    #[error("unauthorized")]
    Unauthorized,
    #[error("too many active keys")]
    TooManyKeys,
    #[error("email already belongs to another account")]
    EmailTaken,
    #[error("poem data could not be parsed")]
    PoemParse,
    #[error("database error")]
    Database(String),
    #[error("search error: {0}")]
    Search(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("too many requests")]
    TooManyRequests,
    #[error("{0:?}")]
    Route(RouteProblem),
}

#[derive(Clone)]
struct Problem {
    status: StatusCode,
    code: &'static str,
    title: &'static str,
    detail: String,
    error: Option<Arc<AppError>>,
}

impl AppError {
    fn problem(&self) -> Problem {
        let contract = |status, code, title, detail: &str| Problem {
            status,
            code,
            title,
            detail: detail.to_string(),
            error: None,
        };
        match self {
            AppError::NotFound(resource) => contract(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Resource not found",
                resource.not_found_detail(),
            ),
            AppError::Unauthorized => contract(
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Unauthorized",
                "Unauthorized",
            ),
            AppError::TooManyKeys => contract(
                StatusCode::CONFLICT,
                "TOO_MANY_KEYS",
                "Too many active keys",
                "Revoke an existing key before creating another",
            ),
            AppError::EmailTaken => contract(
                StatusCode::CONFLICT,
                "EMAIL_TAKEN",
                "Email already belongs to another account",
                "This email is already used by a different account",
            ),
            AppError::PoemParse => contract(
                StatusCode::INTERNAL_SERVER_ERROR,
                "POEM_PARSE_ERROR",
                "Poem data could not be parsed",
                "Unable to load poem",
            ),
            AppError::Search(_) | AppError::Database(_) => contract(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_SERVER_ERROR",
                "Internal server error",
                "Internal server error",
            ),
            AppError::Unavailable(_) => contract(
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE",
                "Service unavailable",
                "Temporarily unavailable, try again shortly",
            ),
            AppError::TooManyRequests => contract(
                StatusCode::TOO_MANY_REQUESTS,
                "TOO_MANY_REQUESTS",
                "Too many requests",
                "Too many requests",
            ),
            AppError::Route(route) => Problem {
                status: route.status,
                code: route.code,
                title: route.title,
                detail: route.detail.clone(),
                error: None,
            },
        }
    }

    pub fn render_at(&self, instance: &str) -> Response {
        render(&self.problem(), instance)
    }
}

impl From<RouteProblem> for AppError {
    fn from(problem: RouteProblem) -> Self {
        AppError::Route(problem)
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Database(cause) => AppError::Database(cause),
            StoreError::Search(cause) => AppError::Search(cause),
            StoreError::Unavailable(cause) => AppError::Unavailable(cause),
        }
    }
}

impl From<PoemError> for AppError {
    fn from(error: PoemError) -> Self {
        match error {
            PoemError::PoetNotShown => AppError::NotFound(Resource::Poet),
            PoemError::MissingVerses => AppError::PoemParse,
            PoemError::NoRandomPoem => RouteProblem::internal("Failed to fetch random poem").into(),
            PoemError::Store(error) => error.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            AppError::Database(cause) => {
                tracing::error!(stage = "query", error = %cause);
            }
            AppError::Search(cause) => {
                tracing::error!(stage = "search", error = %cause);
            }
            AppError::Unavailable(cause) => {
                tracing::warn!(stage = "unavailable", error = %cause);
            }
            AppError::NotFound(_)
            | AppError::Unauthorized
            | AppError::TooManyKeys
            | AppError::EmailTaken
            | AppError::PoemParse
            | AppError::TooManyRequests
            | AppError::Route(_) => {}
        }
        let mut problem = self.problem();
        let status = problem.status;
        if status.is_server_error() {
            problem.error = Some(Arc::new(self));
        }
        let mut response = status.into_response();
        response.extensions_mut().insert(problem);
        response
    }
}

fn type_url(code: &str) -> String {
    format!(
        "{PROD_SITE_URL}/errors/{}",
        code.to_lowercase().replace('_', "-")
    )
}

#[derive(Serialize)]
struct ProblemBody<'a> {
    #[serde(rename = "type")]
    type_url: String,
    title: &'a str,
    status: u16,
    code: &'a str,
    instance: &'a str,
    detail: &'a str,
}

#[expect(
    clippy::expect_used,
    reason = "a problem document built from fixed strings is always serializable"
)]
fn problem_body(problem: &Problem, instance: &str) -> String {
    serde_json::to_string(&ProblemBody {
        type_url: type_url(problem.code),
        title: problem.title,
        status: problem.status.as_u16(),
        code: problem.code,
        instance,
        detail: &problem.detail,
    })
    .expect("a problem document is always serializable")
}

fn render(problem: &Problem, instance: &str) -> Response {
    let mut response = (problem.status, problem_body(problem, instance)).into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(PROBLEM_JSON));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(NO_STORE_CACHE_CONTROL),
    );
    if problem.status == StatusCode::SERVICE_UNAVAILABLE {
        response.headers_mut().insert(
            header::RETRY_AFTER,
            HeaderValue::from(UNAVAILABLE_RETRY_AFTER_SECONDS),
        );
    }
    response
}

pub async fn layer(request: Request, next: Next) -> Response {
    let instance = request.uri().path().to_string();
    let method = request.method().to_string();
    let response = next.run(request).await;
    let Some(problem) = response.extensions().get::<Problem>().cloned() else {
        return response;
    };
    if let Some(error) = &problem.error {
        sentry::capture(error, problem.code, &method, &instance);
    }
    let mut rendered = render(&problem, &instance);
    for name in [
        RATE_LIMIT_LIMIT_HEADER,
        RATE_LIMIT_REMAINING_HEADER,
        RATE_LIMIT_RESET_HEADER,
    ] {
        if let Some(value) = response.headers().get(name) {
            rendered.headers_mut().insert(name, value.clone());
        }
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_the_type_url_from_the_code() {
        assert_eq!(
            type_url("NOT_FOUND"),
            "https://qafiyah.com/errors/not-found"
        );
        assert_eq!(
            type_url("INTERNAL_SERVER_ERROR"),
            "https://qafiyah.com/errors/internal-server-error"
        );
    }

    #[test]
    fn every_resource_has_the_contract_message() {
        assert_eq!(Resource::Meter.not_found_detail(), "Meter not found");
        assert_eq!(
            Resource::Collection.not_found_detail(),
            "Collection not found"
        );
        assert_eq!(Resource::PoemType.not_found_detail(), "Poem type not found");
    }

    fn body(error: AppError, instance: &str) -> String {
        problem_body(&error.problem(), instance)
    }

    #[test]
    fn renders_every_problem_in_the_declared_member_order() {
        assert_eq!(
            body(AppError::NotFound(Resource::Meter), "/v1/meters/zzzz"),
            r#"{"type":"https://qafiyah.com/errors/not-found","title":"Resource not found","status":404,"code":"NOT_FOUND","instance":"/v1/meters/zzzz","detail":"Meter not found"}"#
        );
        assert_eq!(
            body(
                AppError::Route(RouteProblem::no_route(&Method::GET, "/v1/nope")),
                "/v1/nope"
            ),
            r#"{"type":"https://qafiyah.com/errors/not-found","title":"Resource not found","status":404,"code":"NOT_FOUND","instance":"/v1/nope","detail":"No route matches GET /v1/nope"}"#
        );
    }

    #[test]
    fn names_the_method_and_path_that_did_not_match() {
        let problem = RouteProblem::no_route(&Method::GET, "/v1/nope");
        assert_eq!(problem.detail, "No route matches GET /v1/nope");
    }
}
