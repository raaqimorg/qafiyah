use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::ConnectInfo;
use axum::http::request::Parts;
use ipnet::IpNet;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::{Api, Failure, Reply};
use crate::forwarding::caller_address;
use crate::metrics::{Metrics, Outcome, Tool};
use crate::results::{self, Kind, Subject};
use crate::slugs::{FOUR_LETTER_PATTERN, TERM_PATTERN, is_poem_slug, is_term_slug};

const INSTRUCTIONS: &str = "Qafiyah (qafiyah.com) is an open catalog of Arabic poetry: poems, poets, eras, meters, rhymes, themes, collections, and the feelings that verses express. Search in Arabic. Quote verses exactly as the tools give them, never from memory, and give the qafiyah.com url of each poem or poet you quote.";

type Query = Vec<(&'static str, String)>;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    Poems,
    Poets,
}

impl SearchType {
    fn wire(&self) -> &'static str {
        match self {
            Self::Poems => "poems",
            Self::Poets => "poets",
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchInput {
    #[serde(default)]
    #[schemars(
        with = "String",
        description = "The search text, in Arabic. Leave it out to browse with filters only."
    )]
    pub q: Option<String>,
    #[serde(default)]
    #[schemars(description = "Which sections to search: poems, poets, or both.")]
    pub types: Vec<SearchType>,
    #[serde(default)]
    #[schemars(with = "u32", description = "The page of poem results, from 1.")]
    pub poems_page: Option<u32>,
    #[serde(default)]
    #[schemars(with = "u32", description = "The page of poet results, from 1.")]
    pub poets_page: Option<u32>,
    #[serde(default)]
    #[schemars(description = "Poet slugs to filter by.")]
    pub poet_slugs: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Era slugs to filter by.")]
    pub era_slugs: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Meter slugs to filter by.")]
    pub meter_slugs: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Rhyme slugs to filter by.")]
    pub rhyme_slugs: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Theme slugs to filter by.")]
    pub theme_slugs: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Poem type slugs to filter by.")]
    pub poem_type_slugs: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Collection slugs to filter by.")]
    pub collection_slugs: Vec<String>,
    #[serde(default)]
    #[schemars(
        with = "bool",
        description = "Match the exact words, not related forms."
    )]
    pub exact: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPoemsInput {
    #[serde(default)]
    #[schemars(with = "u32", description = "The page, from 1.")]
    pub page: Option<u32>,
    #[serde(default)]
    #[schemars(description = "Poet slugs to filter by.")]
    pub poet: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Era slugs to filter by.")]
    pub era: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Theme slugs to filter by.")]
    pub theme: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Meter slugs to filter by.")]
    pub meter: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Rhyme slugs to filter by.")]
    pub rhyme: Vec<String>,
    #[serde(default)]
    #[schemars(description = "Collection slugs to filter by.")]
    pub collection: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PoemInput {
    #[schemars(
        description = "The poem's four-letter slug, such as Abcd.",
        regex(pattern = FOUR_LETTER_PATTERN)
    )]
    pub slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPoetsInput {
    #[serde(default)]
    #[schemars(with = "u32", description = "The page, from 1.")]
    pub page: Option<u32>,
    #[serde(default)]
    #[schemars(with = "String", description = "An era slug to filter by.")]
    pub era: Option<String>,
    #[serde(default)]
    #[schemars(with = "String", description = "Part of the poet's name, in Arabic.")]
    pub q: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PoetInput {
    #[schemars(
        description = "The poet's four-letter slug, such as Abcd.",
        regex(pattern = FOUR_LETTER_PATTERN)
    )]
    pub slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TermKind {
    Eras,
    Meters,
    Rhymes,
    Themes,
    PoemTypes,
    Collections,
}

impl TermKind {
    fn segment(&self) -> &'static str {
        match self {
            Self::Eras => "eras",
            Self::Meters => "meters",
            Self::Rhymes => "rhymes",
            Self::Themes => "themes",
            Self::PoemTypes => "poem-types",
            Self::Collections => "collections",
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTermsInput {
    #[schemars(description = "Which list of terms to get.")]
    pub kind: TermKind,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FeelingVersesInput {
    #[schemars(
        description = "The feeling's slug, from list_feelings.",
        regex(pattern = TERM_PATTERN)
    )]
    pub slug: String,
    #[serde(default)]
    #[schemars(with = "u32", description = "The page, from 1.")]
    pub page: Option<u32>,
}

#[derive(Clone)]
pub struct Catalog {
    api: Api,
    trusted: Arc<[IpNet]>,
    metrics: Metrics,
    max_poem_chars: usize,
}

fn success(value: &Value) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(value.to_string())])
}

fn error(text: String) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(text)])
}

fn outcome_of(failure: &Failure) -> Outcome {
    match failure {
        Failure::RateLimited { .. } => Outcome::Refused,
        Failure::BadRequest(_) => Outcome::BadInput,
        Failure::NotFound => Outcome::NotFound,
        Failure::Unavailable => Outcome::Unavailable,
    }
}

fn shaped(body: &str, shape: impl FnOnce(&mut Value)) -> Option<Value> {
    let mut value: Value = serde_json::from_str(body).ok()?;
    shape(&mut value);
    Some(value)
}

fn push(query: &mut Query, name: &'static str, value: Option<String>) {
    if let Some(value) = value {
        query.push((name, value));
    }
}

fn push_all(query: &mut Query, name: &'static str, values: &[String]) {
    query.extend(values.iter().map(|value| (name, value.clone())));
}

fn moved(tool: Tool, subject: Subject, location: &str) -> (Outcome, CallToolResult) {
    let kind = match subject {
        Subject::Poem => Some(Kind::Poem),
        Subject::Poet => Some(Kind::Poet),
        Subject::Term | Subject::Feeling | Subject::Catalog => None,
    };
    let slug = location
        .split('?')
        .next()
        .and_then(|path| path.rsplit('/').next())
        .filter(|slug| is_poem_slug(slug));
    match (kind, slug) {
        (Some(kind), Some(slug)) => (
            Outcome::Ok,
            success(&results::moved(kind, slug, tool.name())),
        ),
        _ => (
            Outcome::Unavailable,
            error(results::failure_text(&Failure::Unavailable, subject)),
        ),
    }
}

impl Catalog {
    pub fn new(api: Api, trusted: Arc<[IpNet]>, metrics: Metrics, max_poem_chars: usize) -> Self {
        Self {
            api,
            trusted,
            metrics,
            max_poem_chars,
        }
    }

    fn refuse(&self, tool: Tool, slug: &str, subject: Subject) -> CallToolResult {
        self.metrics.observe_call(tool, Outcome::BadInput, 0.0);
        error(results::bad_slug_text(slug, subject))
    }

    async fn run(
        &self,
        tool: Tool,
        subject: Subject,
        parts: &Parts,
        segments: &[&str],
        query: &Query,
        shape: impl FnOnce(&str) -> Option<Value>,
    ) -> CallToolResult {
        let started = Instant::now();
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(address)| address.ip());
        let caller = caller_address(&parts.headers, peer, &self.trusted);
        let (outcome, result) = match self.api.get(segments, query, caller).await {
            Ok(Reply::Found(body)) => match shape(&body) {
                Some(value) => (Outcome::Ok, success(&value)),
                None => (
                    Outcome::Unavailable,
                    error(results::failure_text(&Failure::Unavailable, subject)),
                ),
            },
            Ok(Reply::Moved(location)) => moved(tool, subject, &location),
            Err(failure) => (
                outcome_of(&failure),
                error(results::failure_text(&failure, subject)),
            ),
        };
        if outcome == Outcome::Unavailable {
            tracing::warn!(tool = tool.name(), "the API call failed");
        }
        self.metrics
            .observe_call(tool, outcome, started.elapsed().as_secs_f64());
        result
    }
}

#[tool_router]
impl Catalog {
    #[tool(
        name = "search",
        description = "Search Qafiyah's catalog of Arabic poetry for poems and poets, in Arabic, with optional filters by slug. Get valid filter slugs from list_terms or list_poets first. Quote verses exactly as the result gives them, and give each poem's and poet's url.",
        annotations(read_only_hint = true)
    )]
    async fn search(
        &self,
        Parameters(input): Parameters<SearchInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut query = Query::new();
        push(&mut query, "q", input.q);
        for kind in &input.types {
            query.push(("types", kind.wire().to_string()));
        }
        push(
            &mut query,
            "poemsPage",
            input.poems_page.map(|page| page.to_string()),
        );
        push(
            &mut query,
            "poetsPage",
            input.poets_page.map(|page| page.to_string()),
        );
        push_all(&mut query, "poetSlugs", &input.poet_slugs);
        push_all(&mut query, "eraSlugs", &input.era_slugs);
        push_all(&mut query, "meterSlugs", &input.meter_slugs);
        push_all(&mut query, "rhymeSlugs", &input.rhyme_slugs);
        push_all(&mut query, "themeSlugs", &input.theme_slugs);
        push_all(&mut query, "poemTypeSlugs", &input.poem_type_slugs);
        push_all(&mut query, "collectionSlugs", &input.collection_slugs);
        push(
            &mut query,
            "exact",
            input.exact.map(|exact| exact.to_string()),
        );
        Ok(self
            .run(
                Tool::Search,
                Subject::Catalog,
                &parts,
                &["search"],
                &query,
                |body| shaped(body, results::shape_search),
            )
            .await)
    }

    #[tool(
        name = "list_poems",
        description = "List poems in Qafiyah's catalog, filtered by poet, era, theme, meter, rhyme, or collection slugs. Get valid slugs from list_terms, list_poets, or search first. Give each poem's url.",
        annotations(read_only_hint = true)
    )]
    async fn list_poems(
        &self,
        Parameters(input): Parameters<ListPoemsInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut query = Query::new();
        push(&mut query, "page", input.page.map(|page| page.to_string()));
        push_all(&mut query, "poet", &input.poet);
        push_all(&mut query, "era", &input.era);
        push_all(&mut query, "theme", &input.theme);
        push_all(&mut query, "meter", &input.meter);
        push_all(&mut query, "rhyme", &input.rhyme);
        push_all(&mut query, "collection", &input.collection);
        Ok(self
            .run(
                Tool::ListPoems,
                Subject::Catalog,
                &parts,
                &["poems"],
                &query,
                |body| shaped(body, results::shape_poem_list),
            )
            .await)
    }

    #[tool(
        name = "get_poem",
        description = "Get one poem of Qafiyah's catalog by its four-letter slug, with its verses, poet, era, meter, rhyme, and theme. Get valid slugs from search, list_poems, or random_poem first. Quote verses exactly as the result gives them, and give the poem's url.",
        annotations(read_only_hint = true)
    )]
    async fn get_poem(
        &self,
        Parameters(input): Parameters<PoemInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        if !is_poem_slug(&input.slug) {
            return Ok(self.refuse(Tool::GetPoem, &input.slug, Subject::Poem));
        }
        let max_chars = self.max_poem_chars;
        Ok(self
            .run(
                Tool::GetPoem,
                Subject::Poem,
                &parts,
                &["poems", &input.slug],
                &Query::new(),
                |body| shaped(body, |value| results::shape_poem(value, max_chars)),
            )
            .await)
    }

    #[tool(
        name = "random_poem",
        description = "Pick a random poem from Qafiyah's catalog. It answers the poem's slug and url: call get_poem with the slug for its verses, and quote them exactly.",
        annotations(read_only_hint = true)
    )]
    async fn random_poem(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let query: Query = vec![("option", "slug".to_string())];
        Ok(self
            .run(
                Tool::RandomPoem,
                Subject::Poem,
                &parts,
                &["poems", "random"],
                &query,
                |body| {
                    let slug = body.trim();
                    is_poem_slug(slug)
                        .then(|| json!({ "slug": slug, "url": results::link(Kind::Poem, slug) }))
                },
            )
            .await)
    }

    #[tool(
        name = "list_poets",
        description = "List poets in Qafiyah's catalog, optionally by era slug or by part of the name in Arabic. Get valid era slugs from list_terms first. Give each poet's url.",
        annotations(read_only_hint = true)
    )]
    async fn list_poets(
        &self,
        Parameters(input): Parameters<ListPoetsInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut query = Query::new();
        push(&mut query, "page", input.page.map(|page| page.to_string()));
        push(&mut query, "era", input.era);
        push(&mut query, "q", input.q);
        Ok(self
            .run(
                Tool::ListPoets,
                Subject::Catalog,
                &parts,
                &["poets"],
                &query,
                |body| shaped(body, results::shape_poet_list),
            )
            .await)
    }

    #[tool(
        name = "get_poet",
        description = "Get one poet of Qafiyah's catalog by the four-letter slug: name, era, biography, and poem count. Get valid slugs from list_poets or search first. Give the poet's url.",
        annotations(read_only_hint = true)
    )]
    async fn get_poet(
        &self,
        Parameters(input): Parameters<PoetInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        if !is_poem_slug(&input.slug) {
            return Ok(self.refuse(Tool::GetPoet, &input.slug, Subject::Poet));
        }
        Ok(self
            .run(
                Tool::GetPoet,
                Subject::Poet,
                &parts,
                &["poets", &input.slug],
                &Query::new(),
                |body| shaped(body, results::shape_poet),
            )
            .await)
    }

    #[tool(
        name = "list_terms",
        description = "List one kind of term in Qafiyah's catalog (eras, meters, rhymes, themes, poem types, or collections), with each term's slug and counts. Use the slugs to filter list_poems, list_poets, and search.",
        annotations(read_only_hint = true)
    )]
    async fn list_terms(
        &self,
        Parameters(input): Parameters<ListTermsInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self
            .run(
                Tool::ListTerms,
                Subject::Term,
                &parts,
                &[input.kind.segment()],
                &Query::new(),
                |body| serde_json::from_str(body).ok(),
            )
            .await)
    }

    #[tool(
        name = "list_feelings",
        description = "List the feelings that Qafiyah classifies verses by, with each feeling's slug. Use a slug with feeling_verses.",
        annotations(read_only_hint = true)
    )]
    async fn list_feelings(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self
            .run(
                Tool::ListFeelings,
                Subject::Feeling,
                &parts,
                &["feelings"],
                &Query::new(),
                |body| serde_json::from_str(body).ok(),
            )
            .await)
    }

    #[tool(
        name = "feeling_verses",
        description = "List verses of Qafiyah's catalog that express one feeling, by the feeling's slug. Get valid slugs from list_feelings first. Quote verses exactly as the result gives them, and give each poem's url.",
        annotations(read_only_hint = true)
    )]
    async fn feeling_verses(
        &self,
        Parameters(input): Parameters<FeelingVersesInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        if !is_term_slug(&input.slug) {
            return Ok(self.refuse(Tool::FeelingVerses, &input.slug, Subject::Feeling));
        }
        let mut query = Query::new();
        push(&mut query, "page", input.page.map(|page| page.to_string()));
        Ok(self
            .run(
                Tool::FeelingVerses,
                Subject::Feeling,
                &parts,
                &["feelings", &input.slug, "verses"],
                &query,
                |body| shaped(body, results::shape_feeling_verses),
            )
            .await)
    }
}

#[tool_handler]
impl ServerHandler for Catalog {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("qafiyah", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use axum::extract::ConnectInfo;
    use axum::http::request::Parts;
    use axum::http::{Request, StatusCode};
    use rmcp::handler::server::tool::Extension;
    use rmcp::handler::server::wrapper::Parameters;
    use rmcp::model::CallToolResult;
    use serde_json::{Value, json};

    use super::*;
    use crate::api::{Api, Failure};
    use crate::metrics::Metrics;
    use crate::results::{Subject, failure_text};
    use crate::test_support::{FakeApi, Seen, histogram_count, scrape};

    fn parts() -> Extension<Parts> {
        let (mut parts, ()) = Request::builder()
            .header("cf-connecting-ip", "203.0.113.7")
            .body(())
            .expect("a request")
            .into_parts();
        parts
            .extensions
            .insert(ConnectInfo(SocketAddr::from(([172, 27, 0, 5], 40_000))));
        Extension(parts)
    }

    fn catalog_with(fake: &FakeApi, metrics: Metrics) -> Catalog {
        Catalog::new(
            Api::new(&fake.url, Some("mcp-key".into()), Duration::from_secs(2)).expect("a client"),
            Arc::from(vec!["172.27.0.0/24".parse().expect("a network")]),
            metrics,
            1_000,
        )
    }

    fn catalog(fake: &FakeApi) -> Catalog {
        catalog_with(fake, Metrics::default())
    }

    fn text(result: &CallToolResult) -> String {
        result
            .content
            .first()
            .and_then(|content| content.as_text())
            .map(|content| content.text.clone())
            .unwrap_or_default()
    }

    fn body(result: &CallToolResult) -> Value {
        assert_ne!(result.is_error, Some(true), "{}", text(result));
        serde_json::from_str(&text(result)).expect("a JSON result")
    }

    async fn only_request(fake: &FakeApi) -> Seen {
        let seen = fake.requests().await;
        assert_eq!(seen.len(), 1, "one API call");
        let request = seen.into_iter().next().expect("a request");
        assert_eq!(
            request.headers.get("x-api-key").map(|v| v.as_bytes()),
            Some(&b"mcp-key"[..])
        );
        assert_eq!(
            request
                .headers
                .get("cf-connecting-ip")
                .map(|v| v.as_bytes()),
            Some(&b"203.0.113.7"[..])
        );
        request
    }

    async fn serving(body: Value) -> FakeApi {
        FakeApi::serving(StatusCode::OK, &[], &body.to_string()).await
    }

    #[tokio::test]
    async fn search_calls_the_search_path_and_links_poems_and_poets() {
        let fake = serving(json!({
            "q": "سلام",
            "poems": { "data": [{ "slug": "Abcd", "poet": { "slug": "Wxyz" } }] },
            "poets": { "data": [{ "slug": "Wxyz" }] },
        }))
        .await;
        let input = SearchInput {
            q: Some("سلام".into()),
            types: vec![SearchType::Poems, SearchType::Poets],
            poems_page: Some(2),
            poets_page: None,
            poet_slugs: vec![],
            era_slugs: vec!["jahili".into()],
            meter_slugs: vec![],
            rhyme_slugs: vec![],
            theme_slugs: vec![],
            poem_type_slugs: vec![],
            collection_slugs: vec![],
            exact: Some(true),
        };
        let result = catalog(&fake)
            .search(Parameters(input), parts())
            .await
            .expect("a result");
        let value = body(&result);
        assert_eq!(
            value["poems"]["data"][0]["url"],
            "https://qafiyah.com/poems/Abcd"
        );
        assert_eq!(
            value["poems"]["data"][0]["poet"]["url"],
            "https://qafiyah.com/poets/Wxyz"
        );
        assert_eq!(
            value["poets"]["data"][0]["url"],
            "https://qafiyah.com/poets/Wxyz"
        );
        let request = only_request(&fake).await;
        let (path, query) = request
            .path_and_query
            .split_once('?')
            .expect("a query string");
        assert_eq!(path, "/v1/search");
        let pairs: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect();
        let expected: Vec<(String, String)> = [
            ("q", "سلام"),
            ("types", "poems"),
            ("types", "poets"),
            ("poemsPage", "2"),
            ("eraSlugs", "jahili"),
            ("exact", "true"),
        ]
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect();
        assert_eq!(pairs, expected);
    }

    #[tokio::test]
    async fn list_poems_calls_the_poems_path_and_links_each_poem_and_poet() {
        let fake =
            serving(json!({ "data": [{ "slug": "Abcd", "poet": { "slug": "Wxyz" } }] })).await;
        let input = ListPoemsInput {
            page: Some(2),
            poet: vec![],
            era: vec!["jahili".into(), "abbasi".into()],
            theme: vec![],
            meter: vec![],
            rhyme: vec![],
            collection: vec![],
        };
        let result = catalog(&fake)
            .list_poems(Parameters(input), parts())
            .await
            .expect("a result");
        let value = body(&result);
        assert_eq!(value["data"][0]["url"], "https://qafiyah.com/poems/Abcd");
        assert_eq!(
            value["data"][0]["poet"]["url"],
            "https://qafiyah.com/poets/Wxyz"
        );
        assert_eq!(
            only_request(&fake).await.path_and_query,
            "/v1/poems?page=2&era=jahili&era=abbasi"
        );
    }

    #[tokio::test]
    async fn get_poem_links_the_poem_and_every_poem_and_poet_it_names() {
        let fake = serving(json!({ "data": {
            "slug": "Abcd",
            "poet": { "slug": "Wxyz" },
            "next": { "slug": "Bcde" },
            "prev": { "slug": "Cdef" },
            "recensionOf": { "slug": "Defg" },
            "recensions": [{ "slug": "Efgh" }],
            "relatedPoems": [{ "slug": "Fghi", "poet": { "slug": "Ghij" } }],
            "verseCount": 1,
            "verses": [["a", "b"]],
        }}))
        .await;
        let result = catalog(&fake)
            .get_poem(
                Parameters(PoemInput {
                    slug: "Abcd".into(),
                }),
                parts(),
            )
            .await
            .expect("a result");
        let data = body(&result)["data"].clone();
        for (pointer, url) in [
            ("/url", "https://qafiyah.com/poems/Abcd"),
            ("/poet/url", "https://qafiyah.com/poets/Wxyz"),
            ("/next/url", "https://qafiyah.com/poems/Bcde"),
            ("/prev/url", "https://qafiyah.com/poems/Cdef"),
            ("/recensionOf/url", "https://qafiyah.com/poems/Defg"),
            ("/recensions/0/url", "https://qafiyah.com/poems/Efgh"),
            ("/relatedPoems/0/url", "https://qafiyah.com/poems/Fghi"),
            ("/relatedPoems/0/poet/url", "https://qafiyah.com/poets/Ghij"),
        ] {
            assert_eq!(data.pointer(pointer), Some(&json!(url)), "{pointer}");
        }
        assert_eq!(only_request(&fake).await.path_and_query, "/v1/poems/Abcd");
    }

    #[tokio::test]
    async fn random_poem_answers_a_slug_and_its_link() {
        let fake = FakeApi::serving(StatusCode::OK, &[], "Abcd").await;
        let result = catalog(&fake).random_poem(parts()).await.expect("a result");
        assert_eq!(
            body(&result),
            json!({ "slug": "Abcd", "url": "https://qafiyah.com/poems/Abcd" })
        );
        assert_eq!(
            only_request(&fake).await.path_and_query,
            "/v1/poems/random?option=slug"
        );
    }

    #[tokio::test]
    async fn the_poet_tools_call_their_paths_and_link_each_poet() {
        let list = serving(json!({ "data": [{ "slug": "Wxyz" }] })).await;
        let input = ListPoetsInput {
            page: Some(1),
            era: Some("abbasi".into()),
            q: Some("x".into()),
        };
        let result = catalog(&list)
            .list_poets(Parameters(input), parts())
            .await
            .expect("a result");
        assert_eq!(
            body(&result)["data"][0]["url"],
            "https://qafiyah.com/poets/Wxyz"
        );
        assert_eq!(
            only_request(&list).await.path_and_query,
            "/v1/poets?page=1&era=abbasi&q=x"
        );

        let one = serving(json!({ "data": { "slug": "Wxyz" } })).await;
        let result = catalog(&one)
            .get_poet(
                Parameters(PoetInput {
                    slug: "Wxyz".into(),
                }),
                parts(),
            )
            .await
            .expect("a result");
        assert_eq!(
            body(&result)["data"]["url"],
            "https://qafiyah.com/poets/Wxyz"
        );
        assert_eq!(only_request(&one).await.path_and_query, "/v1/poets/Wxyz");
    }

    #[tokio::test]
    async fn the_term_and_feeling_lists_call_their_paths_and_add_no_links() {
        for (kind, path) in [
            (TermKind::Rhymes, "/v1/rhymes"),
            (TermKind::PoemTypes, "/v1/poem-types"),
        ] {
            let fake = serving(json!({ "data": [{ "slug": "lam" }] })).await;
            let result = catalog(&fake)
                .list_terms(Parameters(ListTermsInput { kind }), parts())
                .await
                .expect("a result");
            assert!(body(&result)["data"][0].get("url").is_none());
            assert_eq!(only_request(&fake).await.path_and_query, path);
        }
        let fake = serving(json!({ "data": [{ "slug": "longing" }] })).await;
        let result = catalog(&fake)
            .list_feelings(parts())
            .await
            .expect("a result");
        assert!(body(&result)["data"][0].get("url").is_none());
        assert_eq!(only_request(&fake).await.path_and_query, "/v1/feelings");
    }

    #[tokio::test]
    async fn feeling_verses_links_each_poem_and_poet() {
        let fake = serving(json!({ "data": [{
            "poem": { "slug": "Abcd" },
            "poet": { "slug": "Wxyz" },
            "parts": ["a", "b"],
        }]}))
        .await;
        let input = FeelingVersesInput {
            slug: "longing".into(),
            page: Some(3),
        };
        let result = catalog(&fake)
            .feeling_verses(Parameters(input), parts())
            .await
            .expect("a result");
        let value = body(&result);
        assert_eq!(
            value["data"][0]["poem"]["url"],
            "https://qafiyah.com/poems/Abcd"
        );
        assert_eq!(
            value["data"][0]["poet"]["url"],
            "https://qafiyah.com/poets/Wxyz"
        );
        assert_eq!(
            only_request(&fake).await.path_and_query,
            "/v1/feelings/longing/verses?page=3"
        );
    }

    #[tokio::test]
    async fn a_slug_outside_its_pattern_makes_no_api_call() {
        let fake = serving(json!({})).await;
        let poem = catalog(&fake)
            .get_poem(
                Parameters(PoemInput {
                    slug: "../../account/sessions".into(),
                }),
                parts(),
            )
            .await
            .expect("a result");
        assert_eq!(poem.is_error, Some(true));
        let feeling = catalog(&fake)
            .feeling_verses(
                Parameters(FeelingVersesInput {
                    slug: "A/b".into(),
                    page: None,
                }),
                parts(),
            )
            .await
            .expect("a result");
        assert_eq!(feeling.is_error, Some(true));
        assert!(fake.requests().await.is_empty());
    }

    #[tokio::test]
    async fn a_moved_poem_answers_the_new_slug_and_link_with_one_call() {
        let fake = FakeApi::serving(
            StatusCode::MOVED_PERMANENTLY,
            &[("location", "/v1/poems/Wxyz")],
            "",
        )
        .await;
        let result = catalog(&fake)
            .get_poem(
                Parameters(PoemInput {
                    slug: "Abcd".into(),
                }),
                parts(),
            )
            .await
            .expect("a result");
        assert_ne!(result.is_error, Some(true));
        let text = text(&result);
        assert!(text.contains("Wxyz"), "{text}");
        assert!(text.contains("https://qafiyah.com/poems/Wxyz"), "{text}");
        assert_eq!(fake.requests().await.len(), 1);
    }

    #[tokio::test]
    async fn api_failures_become_tool_errors() {
        for (status, headers, failure) in [
            (
                StatusCode::TOO_MANY_REQUESTS,
                &[("retry-after", "30")][..],
                Failure::RateLimited {
                    retry_after: Some(30),
                },
            ),
            (StatusCode::BAD_REQUEST, &[][..], Failure::BadRequest(None)),
            (StatusCode::NOT_FOUND, &[][..], Failure::NotFound),
            (
                StatusCode::SERVICE_UNAVAILABLE,
                &[][..],
                Failure::Unavailable,
            ),
        ] {
            let fake = FakeApi::serving(status, headers, "").await;
            let result = catalog(&fake)
                .get_poet(
                    Parameters(PoetInput {
                        slug: "Wxyz".into(),
                    }),
                    parts(),
                )
                .await
                .expect("a result");
            assert_eq!(result.is_error, Some(true), "{status}");
            assert_eq!(
                text(&result),
                failure_text(&failure, Subject::Poet),
                "{status}"
            );
        }
    }

    #[tokio::test]
    async fn each_call_is_recorded_once_with_its_outcome() {
        let fake = serving(json!({ "data": { "slug": "Abcd", "verses": [] } })).await;
        let metrics = Metrics::default();
        let catalog = catalog_with(&fake, metrics.clone());
        let _ok = catalog
            .get_poem(
                Parameters(PoemInput {
                    slug: "Abcd".into(),
                }),
                parts(),
            )
            .await;
        let _bad = catalog
            .get_poem(
                Parameters(PoemInput {
                    slug: "bad!".into(),
                }),
                parts(),
            )
            .await;
        let families = scrape(&metrics).await;
        for outcome in ["ok", "bad_input"] {
            assert_eq!(
                histogram_count(
                    &families,
                    "mcp_tool_call_duration_seconds",
                    &[("tool", "get_poem"), ("outcome", outcome)]
                ),
                1,
                "{outcome}"
            );
        }
    }

    #[test]
    fn slug_inputs_carry_the_api_patterns_in_their_schemas() {
        let tools = Catalog::tool_router().list_all();
        for (name, pattern) in [
            ("get_poem", FOUR_LETTER_PATTERN),
            ("get_poet", FOUR_LETTER_PATTERN),
            ("feeling_verses", TERM_PATTERN),
        ] {
            let tool = tools
                .iter()
                .find(|tool| tool.name == name)
                .expect("the tool");
            let schema = serde_json::to_value(tool.input_schema.as_ref()).expect("a schema");
            assert_eq!(
                schema.pointer("/properties/slug/pattern"),
                Some(&json!(pattern)),
                "{name}"
            );
        }
    }

    fn type_arrays(value: &Value, at: &str, found: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    if key == "type" && child.is_array() {
                        found.push(at.to_string());
                    }
                    type_arrays(child, &format!("{at}/{key}"), found);
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    type_arrays(child, &format!("{at}/{index}"), found);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }

    #[test]
    fn no_input_schema_uses_a_type_array() {
        let mut found = Vec::new();
        for tool in Catalog::tool_router().list_all() {
            let schema = serde_json::to_value(tool.input_schema.as_ref()).expect("a schema");
            type_arrays(&schema, &tool.name, &mut found);
        }
        assert!(
            found.is_empty(),
            "single-type dialects reject these: {found:?}"
        );
    }

    #[test]
    fn every_tool_is_read_only() {
        let tools = Catalog::tool_router().list_all();
        assert_eq!(tools.len(), 9);
        for tool in tools {
            assert_eq!(
                tool.annotations.and_then(|a| a.read_only_hint),
                Some(true),
                "{}",
                tool.name
            );
        }
    }
}
