pub mod account;
pub mod feelings;
pub mod go;
pub mod poems;
pub mod poets;
pub mod search;
pub mod site;
pub mod spec;
pub mod taxonomy;

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;

const REDIRECT_STATUS: StatusCode = StatusCode::FOUND;

pub(crate) fn redirect(location: &str, cache_control: Option<&'static str>) -> Response {
    redirect_with(REDIRECT_STATUS, location, cache_control)
}

pub(crate) fn permanent_redirect(location: &str, cache_control: &'static str) -> Response {
    redirect_with(StatusCode::MOVED_PERMANENTLY, location, Some(cache_control))
}

#[expect(
    clippy::expect_used,
    reason = "a redirect with no body is always valid"
)]
fn redirect_with(
    status: StatusCode,
    location: &str,
    cache_control: Option<&'static str>,
) -> Response {
    let mut response = Response::builder()
        .status(status)
        .body(Body::empty())
        .expect("a redirect with no body is always valid");
    if let Ok(value) = HeaderValue::from_str(location) {
        response.headers_mut().insert(header::LOCATION, value);
    }
    if let Some(cache_control) = cache_control {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(cache_control),
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_permanent_redirect_is_a_301_carrying_its_location_and_cache_policy() {
        let response = permanent_redirect("/v1/poems/TnKK", "private, max-age=300");
        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(response.headers()[header::LOCATION], "/v1/poems/TnKK");
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, max-age=300"
        );
    }
}
