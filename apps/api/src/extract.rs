use axum::extract::FromRequestParts;
use axum::extract::Path;
use axum::http::request::Parts;
use serde::de::DeserializeOwned;

use crate::error::{AppError, RouteProblem};

pub struct SafePath<T>(pub T);

impl<S, T> FromRequestParts<S> for SafePath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match Path::<T>::from_request_parts(parts, state).await {
            Ok(path) => Ok(SafePath(path.0)),
            Err(rejection) => Err(RouteProblem::bad_request(&rejection.body_text()).into()),
        }
    }
}

pub struct SafeQuery<T>(pub T);

pub fn parse_query<T: DeserializeOwned>(raw: &str) -> Result<T, String> {
    let deserializer = serde_html_form::Deserializer::new(form_urlencoded::parse(raw.as_bytes()));
    serde_path_to_error::deserialize(deserializer).map_err(|error| {
        let path = error.path().to_string();
        let reason = error.inner().to_string();
        if path == "." {
            format!("Invalid query string: {reason}")
        } else {
            format!("Invalid query parameter `{path}`: {reason}")
        }
    })
}

impl<S, T> FromRequestParts<S> for SafeQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parse_query(parts.uri.query().unwrap_or_default())
            .map(SafeQuery)
            .map_err(|detail| RouteProblem::bad_request(&detail).into())
    }
}

pub fn invalid_path_slug(reason: &str) -> AppError {
    RouteProblem::bad_request(&format!("Invalid path parameter `slug`: {reason}")).into()
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Sample {
        poet: String,
        poems_page: Option<u32>,
    }

    #[test]
    fn a_valid_query_string_becomes_its_struct() {
        let sample: Sample = parse_query("poet=PAKT&poemsPage=2").unwrap();
        assert_eq!(sample.poet, "PAKT");
        assert_eq!(sample.poems_page, Some(2));
    }

    #[test]
    fn a_refusal_names_the_parameter_that_failed() {
        assert_eq!(
            parse_query::<Sample>("poet=PAKT&poemsPage=x").unwrap_err(),
            "Invalid query parameter `poemsPage`: invalid digit found in string"
        );
        assert_eq!(
            parse_query::<Sample>("poet=PAKT&eras=jahili").unwrap_err(),
            "Invalid query parameter `eras`: unknown field `eras`, expected `poet` or `poemsPage`"
        );
        assert_eq!(
            parse_query::<Sample>("poet=PAKT&poet=oNbs").unwrap_err(),
            "Invalid query parameter `poet`: unsupported value"
        );
    }

    #[test]
    fn an_error_without_a_parameter_names_the_query_string() {
        assert_eq!(
            parse_query::<Sample>("poemsPage=2").unwrap_err(),
            "Invalid query string: missing field `poet`"
        );
    }
}
