use axum::http::StatusCode;
use serde_json::Value;

use crate::http_tests::{app_with, empty_hits};
use crate::test_support::{FakeEs, logged, request};

fn requests(lines: &[Value]) -> Vec<&Value> {
    lines
        .iter()
        .filter(|line| line["message"] == "request")
        .collect()
}

#[tokio::test]
async fn a_fast_success_logs_one_debug_line_with_the_request_span() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let (sent, lines) = logged(app_with(&es), request("GET", "/robots.txt")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    assert_eq!(line["level"], "DEBUG");
    assert_eq!(line["message"], "request");
    assert_eq!(line["status"], 200);
    assert!(line["duration_ms"].is_u64(), "{line}");
    assert_eq!(line["span"]["method"], "GET");
    assert_eq!(line["span"]["route"], "/robots.txt");
    assert_eq!(line["span"]["path"], "/robots.txt");
    assert!(
        line["span"]["request_id"]
            .as_str()
            .is_some_and(|id| id.len() == 36),
        "{line}"
    );
}

#[tokio::test]
async fn a_server_error_is_logged_once_at_error_level() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let (sent, lines) = logged(app_with(&es), request("GET", "/v1/poems/gnNg")).await;
    assert!(sent.status.is_server_error(), "{}", sent.status);
    let requests = requests(&lines);
    assert_eq!(requests.len(), 1, "{lines:?}");
    assert_eq!(requests[0]["level"], "ERROR");
    assert_eq!(requests[0]["status"], sent.status.as_u16());
    assert_eq!(requests[0]["span"]["route"], "/v1/poems/{slug}");
}

#[tokio::test]
async fn a_search_that_found_nothing_is_logged_at_info_with_its_text() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let (sent, lines) = logged(
        app_with(&es),
        request("GET", "/v1/search?q=abc&types=poems"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK);
    let nothing: Vec<_> = lines
        .iter()
        .filter(|line| line["message"] == "found nothing")
        .collect();
    assert_eq!(nothing.len(), 1, "{lines:?}");
    assert_eq!(nothing[0]["level"], "INFO");
    assert_eq!(nothing[0]["span"]["query_text"], "abc");
    assert_eq!(nothing[0]["span"]["result_count"], 0);
    assert_eq!(nothing[0]["span"]["route"], "/v1/search");
    assert_eq!(requests(&lines)[0]["level"], "DEBUG");
}

#[tokio::test]
async fn the_health_check_is_never_logged() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let (sent, lines) = logged(app_with(&es), request("GET", "/healthz")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(lines, Vec::<Value>::new());
}

#[tokio::test]
async fn every_response_carries_the_request_id_it_was_logged_under() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let (sent, lines) = logged(app_with(&es), request("GET", "/robots.txt")).await;
    let id = sent
        .header("x-request-id")
        .expect("an x-request-id")
        .to_string();
    assert_eq!(lines[0]["span"]["request_id"], id);

    let mut incoming = request("GET", "/robots.txt");
    incoming.headers_mut().insert(
        "x-request-id",
        "from-the-caller".parse().expect("a header value"),
    );
    let (kept, kept_lines) = logged(app_with(&es), incoming).await;
    assert_eq!(kept.header("x-request-id"), Some("from-the-caller"));
    assert_eq!(kept_lines[0]["span"]["request_id"], "from-the-caller");
}
