use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use rand::RngExt;
use serde_json::{Map, Value};

use crate::constants::{API_SERVICE_NAME, UNKNOWN_ENVIRONMENT};

const SKIP_PREFIX: &str = "/v1/docs";
const SKIP_EXACT: &str = "/v1/openapi.json";
const SKIP_HEALTH: &str = "/healthz";

const SLOW_REQUEST_MS: u128 = 2000;
const PROD_SAMPLE_RATE: f64 = 0.05;
const SERVER_ERROR: u16 = 500;

#[derive(Clone)]
pub struct LogHandle(Arc<Mutex<Map<String, Value>>>);

impl LogHandle {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Map::new())))
    }

    pub fn set(&self, key: &str, value: impl Into<Value>) {
        if let Ok(mut fields) = self.0.lock() {
            fields.insert(key.to_string(), value.into());
        }
    }

    fn take(&self) -> Map<String, Value> {
        self.0
            .lock()
            .map(|fields| fields.clone())
            .unwrap_or_default()
    }
}

fn request_id() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let mut id = String::with_capacity(36);
    for (position, ch) in hex.chars().enumerate() {
        if matches!(position, 8 | 12 | 16 | 20) {
            id.push('-');
        }
        id.push(ch);
    }
    id
}

fn iso_timestamp() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    iso_from_epoch_millis(i64::try_from(millis).unwrap_or_default())
}

fn iso_from_epoch_millis(millis: i64) -> String {
    let (days, millis_of_day) = (millis.div_euclid(86_400_000), millis.rem_euclid(86_400_000));
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second, milli) = (
        millis_of_day / 3_600_000,
        (millis_of_day / 60_000) % 60,
        (millis_of_day / 1000) % 60,
        millis_of_day % 1000,
    );
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}Z")
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "civil-from-days math runs on a days-since-epoch value bounded far below overflow"
)]
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_position = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_position + 2) / 5 + 1;
    let month = if month_position < 10 {
        month_position + 3
    } else {
        month_position - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

fn environment() -> &'static str {
    static ENVIRONMENT: OnceLock<String> = OnceLock::new();
    ENVIRONMENT.get_or_init(|| {
        std::env::var("ENVIRONMENT").unwrap_or_else(|_| UNKNOWN_ENVIRONMENT.to_string())
    })
}

fn should_emit(
    environment: &str,
    status: u16,
    duration_ms: u128,
    fields: &Map<String, Value>,
) -> bool {
    if environment != "production" {
        return true;
    }
    if status >= SERVER_ERROR || duration_ms > SLOW_REQUEST_MS {
        return true;
    }
    if fields.get("result_count").and_then(Value::as_u64) == Some(0) {
        return true;
    }
    rand::rng().random::<f64>() < PROD_SAMPLE_RATE
}

const SOURCE: &str = "api";

pub fn stage_event(stage: &str, detail: Option<(&str, Value)>) -> String {
    let mut fields = Map::new();
    fields.insert("source".into(), SOURCE.into());
    fields.insert("stage".into(), stage.into());
    if let Some((key, value)) = detail {
        fields.insert(key.to_string(), value);
    }
    Value::Object(fields).to_string()
}

fn should_skip(path: &str) -> bool {
    path == SKIP_EXACT
        || path == SKIP_HEALTH
        || path == SKIP_PREFIX
        || path.starts_with("/v1/docs/")
}

#[expect(
    clippy::print_stdout,
    reason = "the api's request log is one structured line on stdout"
)]
pub async fn layer(mut request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().to_string();
    let skip = should_skip(&path);

    let started = Instant::now();
    let timestamp = iso_timestamp();
    let handle = LogHandle::new();
    request.extensions_mut().insert(handle.clone());

    let response = next.run(request).await;

    if skip {
        return response;
    }

    let duration_ms = started.elapsed().as_millis();
    let status = response.status().as_u16();
    let environment = environment();
    let mut fields = handle.take();
    if !should_emit(environment, status, duration_ms, &fields) {
        return response;
    }

    fields.insert("request_id".into(), request_id().into());
    fields.insert("method".into(), method.into());
    fields.insert("path".into(), path.into());
    fields.insert("timestamp".into(), timestamp.into());
    fields.insert(
        "service".into(),
        serde_json::json!({ "name": API_SERVICE_NAME, "environment": environment }),
    );
    fields.insert(
        "kind".into(),
        if status >= SERVER_ERROR {
            "completed_error"
        } else {
            "completed"
        }
        .into(),
    );
    fields.insert("status_code".into(), status.into());
    fields.insert(
        "duration_ms".into(),
        u64::try_from(duration_ms).unwrap_or(u64::MAX).into(),
    );
    println!("{}", Value::Object(fields));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_ids_look_like_v4_uuids() {
        let id = request_id();
        assert_eq!(id.len(), 36);
        assert_eq!(id.as_bytes()[14], b'4');
        assert!(matches!(id.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
        assert_ne!(request_id(), request_id());
    }

    #[test]
    fn formats_instants_the_way_javascript_does() {
        assert_eq!(iso_from_epoch_millis(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            iso_from_epoch_millis(1_000_000_000_000),
            "2001-09-09T01:46:40.000Z"
        );
        assert_eq!(
            iso_from_epoch_millis(1_700_000_000_000),
            "2023-11-14T22:13:20.000Z"
        );
        assert_eq!(
            iso_from_epoch_millis(951_782_400_000),
            "2000-02-29T00:00:00.000Z"
        );
        assert_eq!(
            iso_from_epoch_millis(253_402_300_799_999),
            "9999-12-31T23:59:59.999Z"
        );
    }

    #[test]
    fn a_stage_line_stays_parseable_whatever_the_error_says() {
        let line = stage_event(
            "query",
            Some(("error", r#"column "slug" does not exist"#.into())),
        );
        let parsed: Value = serde_json::from_str(&line).expect("a log line must be JSON");
        assert_eq!(parsed["source"], "api");
        assert_eq!(parsed["stage"], "query");
        assert_eq!(parsed["error"], r#"column "slug" does not exist"#);
    }

    #[test]
    fn a_stage_line_escapes_its_key_and_stage_too() {
        let line = stage_event("qu\"ery", Some(("er\"ror", "plain".into())));
        let parsed: Value = serde_json::from_str(&line).expect("a log line must be JSON");
        assert_eq!(parsed["qu\"ery"], Value::Null);
        assert_eq!(parsed["stage"], "qu\"ery");
        assert_eq!(parsed["er\"ror"], "plain");
    }

    #[test]
    fn a_stage_line_carries_a_non_string_detail_unquoted() {
        let line = stage_event("ready", Some(("port", 8787.into())));
        let parsed: Value = serde_json::from_str(&line).expect("a log line must be JSON");
        assert_eq!(parsed["port"], 8787);
    }

    #[test]
    fn skips_the_document_and_the_reference_page() {
        assert!(should_skip("/v1/openapi.json"));
        assert!(should_skip("/v1/docs"));
        assert!(should_skip("/v1/docs/anything"));
        assert!(!should_skip("/v1/poems"));
        assert!(!should_skip("/v1/search"));
        assert!(!should_skip("/v1/poems/random"));
        assert!(!should_skip("/v1/docsXYZ"));
    }

    #[test]
    fn skips_the_health_check_that_the_probes_and_docker_call_every_few_seconds() {
        assert!(should_skip("/healthz"));
        assert!(!should_skip("/healthz/x"));
        assert!(!should_skip("/v1/healthz"));
    }

    #[test]
    fn outside_production_every_line_is_emitted() {
        assert!(should_emit("unknown", 200, 1, &Map::new()));
        assert!(should_emit("development", 200, 1, &Map::new()));
    }

    #[test]
    fn production_always_keeps_errors_slow_requests_and_empty_results() {
        assert!(should_emit("production", 500, 1, &Map::new()));
        assert!(should_emit(
            "production",
            200,
            SLOW_REQUEST_MS + 1,
            &Map::new()
        ));
        let mut empty = Map::new();
        empty.insert("result_count".into(), 0.into());
        assert!(should_emit("production", 200, 1, &empty));
    }
}
