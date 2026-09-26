use serde::Serialize;
use utoipa::openapi::path::ParameterIn;
use utoipa::openapi::{ContactBuilder, OpenApi as Document, RefOr, Schema, ServerBuilder};
use utoipa::{Modify, OpenApi, ToSchema};

use crate::constants::{API_V1_PREFIX, MAX_FILTER_SLUGS, PROD_SITE_URL, SITE_NAME_EN};

#[derive(Serialize, ToSchema)]
#[schema(value_type = String, pattern = "^[a-zA-Z]{4}$")]
pub struct FourLetterSlug(String);

#[derive(Serialize, ToSchema)]
#[schema(value_type = String, pattern = "^[a-z][a-z-]*$")]
pub struct TransliteratedSlug(String);

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SearchTypeParam {
    Poems,
    Poets,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum NavScopeParam {
    Theme,
    Meter,
    Rhyme,
    Collection,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExactFlag {
    True,
    False,
}

#[derive(Serialize, ToSchema)]
pub struct ProblemDetail {
    #[schema(format = "uri", example = "https://qafiyah.com/errors/not-found")]
    pub r#type: String,
    #[schema(example = "Resource not found")]
    pub title: String,
    #[schema(example = 404)]
    pub status: i32,
    #[schema(example = "NOT_FOUND")]
    pub code: String,
    #[schema(example = "/v1/meters/zzzz")]
    pub instance: String,
    #[schema(example = "Meter not found")]
    pub detail: String,
}

#[derive(utoipa::IntoResponses)]
pub enum ListErrors {
    #[response(
        status = 429,
        description = "Too many requests",
        headers(
            ("x-ratelimit-limit" = String, description = "Requests allowed per hour"),
            ("x-ratelimit-remaining" = String, description = "Requests left in the current hour"),
            ("x-ratelimit-reset" = String, description = "Unix seconds at which the window resets"),
            ("retry-after" = String, description = "Seconds until the window resets"),
        )
    )]
    TooManyRequests(ProblemDetail),
    #[response(status = 500, description = "Internal server error")]
    Internal(ProblemDetail),
}

#[derive(utoipa::IntoResponses)]
pub enum FilteredListErrors {
    #[response(status = 400, description = "Input validation failed")]
    BadRequest(ProblemDetail),
    #[response(
        status = 429,
        description = "Too many requests",
        headers(
            ("x-ratelimit-limit" = String, description = "Requests allowed per hour"),
            ("x-ratelimit-remaining" = String, description = "Requests left in the current hour"),
            ("x-ratelimit-reset" = String, description = "Unix seconds at which the window resets"),
            ("retry-after" = String, description = "Seconds until the window resets"),
        )
    )]
    TooManyRequests(ProblemDetail),
    #[response(status = 500, description = "Internal server error")]
    Internal(ProblemDetail),
}

#[derive(utoipa::IntoResponses)]
pub enum LookupErrors {
    #[response(status = 400, description = "Input validation failed")]
    BadRequest(ProblemDetail),
    #[response(status = 404, description = "Not found")]
    NotFound(ProblemDetail),
    #[response(
        status = 429,
        description = "Too many requests",
        headers(
            ("x-ratelimit-limit" = String, description = "Requests allowed per hour"),
            ("x-ratelimit-remaining" = String, description = "Requests left in the current hour"),
            ("x-ratelimit-reset" = String, description = "Unix seconds at which the window resets"),
            ("retry-after" = String, description = "Seconds until the window resets"),
        )
    )]
    TooManyRequests(ProblemDetail),
    #[response(status = 500, description = "Internal server error")]
    Internal(ProblemDetail),
}

pub fn finish(doc: &mut Document) {
    doc.info.contact = Some(
        ContactBuilder::new()
            .name(Some(SITE_NAME_EN))
            .url(Some(PROD_SITE_URL))
            .build(),
    );
    doc.servers = Some(vec![ServerBuilder::new().url(API_V1_PREFIX).build()]);
    cap_facet_arrays(doc);
    ProblemContentType.modify(doc);
}

const FACET_ITEM_SCHEMAS: [&str; 2] = ["FourLetterSlug", "TransliteratedSlug"];

fn is_facet_array(array: &utoipa::openapi::schema::Array) -> bool {
    let Ok(items) = serde_json::to_value(&array.items) else {
        return false;
    };
    let Some(reference) = items.get("$ref").and_then(serde_json::Value::as_str) else {
        return false;
    };
    FACET_ITEM_SCHEMAS
        .iter()
        .any(|name| reference == format!("#/components/schemas/{name}"))
}

fn cap_facet_arrays(doc: &mut Document) {
    for item in doc.paths.paths.values_mut() {
        let Some(operation) = item.get.as_mut() else {
            continue;
        };
        let Some(parameters) = operation.parameters.as_mut() else {
            continue;
        };
        for parameter in parameters {
            if parameter.parameter_in != ParameterIn::Query {
                continue;
            }
            let Some(RefOr::T(Schema::Array(array))) = parameter.schema.as_mut() else {
                continue;
            };
            if !is_facet_array(array) {
                continue;
            }
            array.max_items = Some(MAX_FILTER_SLUGS);
            array.default = Some(serde_json::json!([]));
        }
    }
}

pub struct ProblemContentType;

impl Modify for ProblemContentType {
    fn modify(&self, openapi: &mut Document) {
        for item in openapi.paths.paths.values_mut() {
            let operations = [
                item.get.as_mut(),
                item.put.as_mut(),
                item.post.as_mut(),
                item.delete.as_mut(),
                item.options.as_mut(),
                item.head.as_mut(),
                item.patch.as_mut(),
                item.trace.as_mut(),
            ];
            for operation in operations.into_iter().flatten() {
                for (status, response) in operation.responses.responses.iter_mut() {
                    if status.parse::<u16>().is_ok_and(|code| code < 400) {
                        continue;
                    }
                    let RefOr::T(response) = response else {
                        continue;
                    };
                    if let Some(body) = response.content.shift_remove("application/json") {
                        response
                            .content
                            .insert("application/problem+json".to_string(), body);
                    }
                }
            }
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Qafiyah API",
        version = "1.0.0",
        description = "Public, read-only REST API for the Qafiyah Arabic classical-poetry catalog.

Browse poems and poets and filter them by the taxonomies they belong to: eras, meters, rhymes, themes, and collections. Full-text search spans both poems and poets.

Conventions:
- All endpoints are GET and require no authentication.
- Resources are addressed by stable `slug` identifiers returned in list responses.
- List endpoints are paginated via a 1-based `page` query param and return a `pagination` block.
- Multi-select filters are repeatable array params, e.g. `?era=abbasi&era=andalusi`.
- Errors follow RFC 9457 problem details and are served as `application/problem+json`.
- Requests are rate limited. Rate-limited responses carry `x-ratelimit-limit`, `x-ratelimit-remaining`, and `x-ratelimit-reset`.",
        license(name = "MIT"),
    ),
    tags(
        (name = "poems", description = "Browse, filter, and retrieve poems and their full verse text."),
        (name = "poets", description = "Browse and retrieve poets, searchable by name and filterable by era."),
        (name = "search", description = "Full-text search across poems and poets with facet filters."),
        (name = "eras", description = "Literary eras (al-usur al-adabiyya) used to classify poems and poets."),
        (name = "meters", description = "Prosodic meters (al-buhur) of classical Arabic poetry."),
        (name = "rhymes", description = "Rhyme letters (al-qawafi) used to classify poems."),
        (name = "themes", description = "Thematic categories (al-aghrad) of poems."),
        (name = "collections", description = "Curated collections (al-dawawin) of poems."),
    ),
    components(schemas(
        ProblemDetail,
        FourLetterSlug,
        TransliteratedSlug,
        SearchTypeParam,
        NavScopeParam,
        ExactFlag,
    )),
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use crate::document;

    #[test]
    fn declares_every_contract_path() {
        let doc = document();
        let paths: Vec<&str> = doc.paths.paths.keys().map(String::as_str).collect();
        assert_eq!(paths.len(), 18, "expected 18 paths, got {paths:?}");
        for expected in [
            "/collections",
            "/collections/{slug}",
            "/eras",
            "/eras/{slug}",
            "/meters",
            "/meters/{slug}",
            "/poems",
            "/poems/count",
            "/poems/slugs",
            "/poems/{slug}",
            "/poets",
            "/poets/slugs",
            "/poets/{slug}",
            "/rhymes",
            "/rhymes/{slug}",
            "/search",
            "/themes",
            "/themes/{slug}",
        ] {
            assert!(paths.contains(&expected), "missing path {expected}");
        }
    }

    #[test]
    fn the_committed_document_is_current() {
        const COMMITTED: &str = include_str!("../generated/openapi/openapi.json");
        let committed: serde_json::Value =
            serde_json::from_str(COMMITTED).expect("the committed document is malformed");
        let generated = serde_json::to_value(document()).expect("serializable");
        assert_eq!(
            committed, generated,
            "apps/api/generated/openapi/openapi.json is stale; run: bun run openapi:snapshot"
        );
    }

    #[test]
    fn caps_every_facet_array_and_nothing_else() {
        let json = serde_json::to_value(document()).expect("serializable");
        let mut facets = 0;
        for (path, item) in json["paths"].as_object().expect("paths") {
            for parameter in item["get"]["parameters"].as_array().into_iter().flatten() {
                let schema = &parameter["schema"];
                if schema["items"].is_null() {
                    continue;
                }
                let name = parameter["name"].as_str().expect("a name");
                if name == "types" {
                    assert_eq!(schema["maxItems"], 2, "{path} types");
                    assert!(schema["default"].is_null(), "{path} types");
                    continue;
                }
                facets += 1;
                assert_eq!(schema["maxItems"], 100, "{path} {name}");
                assert_eq!(schema["default"], serde_json::json!([]), "{path} {name}");
            }
        }
        assert_eq!(facets, 12, "expected twelve facet params, found {facets}");
    }

    #[test]
    fn labels_error_responses_as_problem_json() {
        let doc = document();
        let json = serde_json::to_value(&doc).expect("serializable");
        let responses = &json["paths"]["/meters/{slug}"]["get"]["responses"];
        assert!(responses["404"]["content"]["application/problem+json"].is_object());
        assert!(responses["404"]["content"]["application/json"].is_null());
        assert!(responses["200"]["content"]["application/json"].is_object());
    }
}
