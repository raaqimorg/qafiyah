use serde::Serialize;
use utoipa::openapi::header::{Header, HeaderBuilder};
use utoipa::openapi::path::ParameterIn;
use utoipa::openapi::schema::{ObjectBuilder, Type};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityRequirement, SecurityScheme};
use utoipa::openapi::{
    Components, ContactBuilder, OpenApi as Document, RefOr, ResponseBuilder, Schema, ServerBuilder,
};
use utoipa::{Modify, OpenApi, ToSchema};

use crate::constants::{
    API_KEY_HEADER, API_V1_PREFIX, MAX_FILTER_SLUGS, PROD_SITE_URL, RATE_LIMIT_LIMIT_HEADER,
    RATE_LIMIT_REMAINING_HEADER, RATE_LIMIT_RESET_HEADER, SITE_NAME_EN,
};
use crate::params::{
    ExactFlag, FourLetterSlug, RandomPoemOptionParam, SearchTypeParam, TransliteratedSlug,
};

#[derive(Serialize, ToSchema)]
pub struct ProblemDetail {
    /// A URI identifying the kind of error. It is an identifier and does not resolve to a page.
    #[schema(format = "uri", example = "https://qafiyah.com/errors/not-found")]
    pub r#type: String,
    /// A short summary of the kind of error.
    #[schema(example = "Resource not found")]
    pub title: String,
    /// The HTTP status code.
    #[schema(example = 404)]
    pub status: i32,
    /// A stable machine-readable code, such as `NOT_FOUND`, `BAD_REQUEST`, `TOO_MANY_REQUESTS`, or `SERVICE_UNAVAILABLE`.
    #[schema(example = "NOT_FOUND")]
    pub code: String,
    /// The path of the request that failed.
    #[schema(example = "/v1/meters/zzzz")]
    pub instance: String,
    /// What went wrong with this request, in English.
    #[schema(example = "Meter not found")]
    pub detail: String,
}

#[derive(utoipa::IntoResponses)]
pub enum ListErrors {
    #[response(
        status = 429,
        description = "Too many requests",
        headers(
            ("x-ratelimit-limit" = i64, description = "Requests allowed per hour"),
            ("x-ratelimit-remaining" = i64, description = "Requests left in the current hour"),
            ("x-ratelimit-reset" = i64, description = "Unix seconds at which the window resets"),
            ("retry-after" = i64, description = "Seconds until the window resets"),
        )
    )]
    TooManyRequests(ProblemDetail),
    #[response(status = 500, description = "Internal server error")]
    Internal(ProblemDetail),
    #[response(
        status = 503,
        description = "Temporarily unavailable: the database or search index did not answer in time",
        headers(
            ("retry-after" = i64, description = "Seconds to wait before retrying"),
        )
    )]
    Unavailable(ProblemDetail),
}

#[derive(utoipa::IntoResponses)]
pub enum FilteredListErrors {
    #[response(status = 400, description = "Input validation failed")]
    BadRequest(ProblemDetail),
    #[response(
        status = 429,
        description = "Too many requests",
        headers(
            ("x-ratelimit-limit" = i64, description = "Requests allowed per hour"),
            ("x-ratelimit-remaining" = i64, description = "Requests left in the current hour"),
            ("x-ratelimit-reset" = i64, description = "Unix seconds at which the window resets"),
            ("retry-after" = i64, description = "Seconds until the window resets"),
        )
    )]
    TooManyRequests(ProblemDetail),
    #[response(status = 500, description = "Internal server error")]
    Internal(ProblemDetail),
    #[response(
        status = 503,
        description = "Temporarily unavailable: the database or search index did not answer in time",
        headers(
            ("retry-after" = i64, description = "Seconds to wait before retrying"),
        )
    )]
    Unavailable(ProblemDetail),
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
            ("x-ratelimit-limit" = i64, description = "Requests allowed per hour"),
            ("x-ratelimit-remaining" = i64, description = "Requests left in the current hour"),
            ("x-ratelimit-reset" = i64, description = "Unix seconds at which the window resets"),
            ("retry-after" = i64, description = "Seconds until the window resets"),
        )
    )]
    TooManyRequests(ProblemDetail),
    #[response(status = 500, description = "Internal server error")]
    Internal(ProblemDetail),
    #[response(
        status = 503,
        description = "Temporarily unavailable: the database or search index did not answer in time",
        headers(
            ("retry-after" = i64, description = "Seconds to wait before retrying"),
        )
    )]
    Unavailable(ProblemDetail),
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
    declare_api_key(doc);
    declare_response_headers(doc);
}

const RATE_LIMIT_HEADERS: [(&str, &str); 3] = [
    (RATE_LIMIT_LIMIT_HEADER, "Requests allowed per hour"),
    (
        RATE_LIMIT_REMAINING_HEADER,
        "Requests left in the current hour",
    ),
    (
        RATE_LIMIT_RESET_HEADER,
        "Unix seconds at which the window resets",
    ),
];

fn header(schema_type: Type, description: &str) -> RefOr<Header> {
    RefOr::T(
        HeaderBuilder::new()
            .schema(Some(ObjectBuilder::new().schema_type(schema_type)))
            .description(Some(description))
            .build(),
    )
}

fn declare_api_key(doc: &mut Document) {
    doc.components.get_or_insert_with(Components::new).add_security_scheme(
        "apiKey",
        SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
            API_KEY_HEADER,
            "Optional. A free key from https://qafiyah.com/developers raises the rate limit. A key the API does not recognize counts as no key.",
        ))),
    );
    doc.security = Some(vec![
        SecurityRequirement::default(),
        SecurityRequirement::new("apiKey", Vec::<String>::new()),
    ]);
}

fn declare_response_headers(doc: &mut Document) {
    for item in doc.paths.paths.values_mut() {
        let Some(operation) = item.get.as_mut() else {
            continue;
        };
        let responses = &mut operation.responses.responses;
        if let Some(RefOr::T(ok)) = responses.get_mut("200")
            && ok.content.contains_key("application/json")
        {
            ok.headers.insert(
                "ETag".to_string(),
                header(
                    Type::String,
                    "Send back in If-None-Match to get 304 while the data is unchanged",
                ),
            );
            responses.insert(
                "304".to_string(),
                RefOr::T(
                    ResponseBuilder::new()
                        .description("Not modified: the If-None-Match ETag still matches, so the body is empty.")
                        .build(),
                ),
            );
        }
        for response in responses.values_mut() {
            let RefOr::T(response) = response else {
                continue;
            };
            for (name, description) in RATE_LIMIT_HEADERS {
                response
                    .headers
                    .entry(name.to_string())
                    .or_insert_with(|| header(Type::Integer, description));
            }
        }
    }
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
            let RefOr::T(parameter) = parameter else {
                continue;
            };
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
        description = "Public, read-only REST API for Qafiyah, an open catalog of Arabic poetry from the pre-Islamic era to the present: poems with their full verse text, poets, and the eras, meters, rhymes, themes, verse forms, and collections that classify them, with full-text search over poems and poets.

Conventions:
- Every endpoint is GET. Responses are JSON, except `GET /poems/random`, which answers in plain text.
- No key is needed. Without one, a caller gets 60 requests an hour per address (an IPv6 /64 counts as one address, and its /48 shares ten times that). A free key from https://qafiyah.com/developers, sent in the `x-api-key` header, gives 500 an hour and 10 a second, and at most 1,500 an hour from any one address. A key the API does not recognize counts as no key, so check `x-ratelimit-limit` after adding one.
- Every rate-limited response, errors included, carries `x-ratelimit-limit`, `x-ratelimit-remaining`, and `x-ratelimit-reset` (Unix seconds). Past the limit the answer is 429 with `Retry-After`. When the API is briefly overloaded or a backing store does not answer in time, it is 503 with `Retry-After`.
- Poems and poets are addressed by four-letter, case-sensitive slugs (`gnNg`, `PAKT`), and eras, meters, rhymes, themes, verse forms, and collections by lowercase transliterated slugs (`jahili`, `altawil`). Take them from list responses.
- `GET /poems`, `GET /poems/slugs`, `GET /poets`, and `GET /poets/slugs` page with a 1-based `page` and return a `pagination` block. `GET /search` pages its two sections with `poemsPage` and `poetsPage`. The taxonomy lists return every term at once.
- To select several values of one filter, repeat the parameter once per value. `GET /poets` takes a single `era`.
- Unknown query parameters are ignored, so a misspelled filter returns unfiltered results rather than an error.
- JSON responses carry an `ETag`. Send it back in `If-None-Match` to get 304 Not Modified while the data is unchanged.
- Errors are RFC 9457 problem details served as `application/problem+json`, with a stable `code`.

The data is dedicated to the public domain under CC0 1.0. The API's code is MIT licensed: https://github.com/raaqimorg/qafiyah",
        license(name = "CC0 1.0 (data)", identifier = "CC0-1.0"),
    ),
    paths(crate::routes::poems::random),
    tags(
        (name = "poems", description = "Browse, filter, and retrieve poems and their full verse text."),
        (name = "poets", description = "Browse and retrieve poets, searchable by name and filterable by era."),
        (name = "search", description = "Full-text search across poems and poets with facet filters."),
        (name = "eras", description = "Literary eras (al-usur al-adabiyya) used to classify poems and poets."),
        (name = "meters", description = "Prosodic meters (al-buhur) of classical Arabic poetry."),
        (name = "rhymes", description = "Rhyme letters (al-qawafi) used to classify poems."),
        (name = "themes", description = "Thematic categories (al-aghrad) of poems."),
        (name = "collections", description = "Curated collections (al-dawawin) of poems."),
        (name = "poem-types", description = "Verse forms (anwa' al-qasida): classical amudi verse, free verse, and the rest."),
    ),
    components(schemas(
        ProblemDetail,
        FourLetterSlug,
        TransliteratedSlug,
        SearchTypeParam,
        ExactFlag,
        RandomPoemOptionParam,
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
        assert_eq!(paths.len(), 22, "expected 22 paths, got {paths:?}");
        for expected in [
            "/collections",
            "/collections/{slug}",
            "/eras",
            "/eras/{slug}",
            "/meters",
            "/meters/{slug}",
            "/poem-types",
            "/poem-types/{slug}",
            "/poems",
            "/poems/count",
            "/poems/facets",
            "/poems/random",
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
                    assert!(schema["maxItems"].is_null(), "{path} types");
                    assert!(schema["default"].is_null(), "{path} types");
                    continue;
                }
                facets += 1;
                assert_eq!(schema["maxItems"], 100, "{path} {name}");
                assert_eq!(schema["default"], serde_json::json!([]), "{path} {name}");
            }
        }
        assert_eq!(facets, 16, "expected sixteen facet params, found {facets}");
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
