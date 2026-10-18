pub mod config;
pub mod constants;
pub mod log;
pub mod metrics;

#[cfg(test)]
mod test_support;

use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{BoxError, Router};
use tower::ServiceBuilder;
use tower::timeout::TimeoutLayer;
use tower::timeout::error::Elapsed;

use crate::constants::REQUEST_DEADLINE_SECONDS;

async fn deadline_passed(error: BoxError) -> StatusCode {
    if error.is::<Elapsed>() {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub fn with_deadline(router: Router, limit: Duration) -> Router {
    router.layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(deadline_passed))
            .layer(TimeoutLayer::new(limit)),
    )
}

pub fn app() -> Router {
    let routes = Router::new()
        .route("/healthz", get(|| async { StatusCode::OK }))
        .fallback(|| async { StatusCode::NOT_FOUND });
    with_deadline(routes, Duration::from_secs(REQUEST_DEADLINE_SECONDS))
}

pub async fn shutdown_signal() {
    let interrupt = async {
        let _interrupt = tokio::signal::ctrl_c().await;
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    tokio::select! {
        () = interrupt => {},
        () = terminate => {},
    }
    tracing::info!(stage = "draining");
}

pub async fn serve<F>(
    listener: tokio::net::TcpListener,
    app: Router,
    shutdown: F,
) -> std::io::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    use super::*;

    async fn status(app: Router, path: &str) -> StatusCode {
        app.oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("a request"),
        )
        .await
        .expect("the router is infallible")
        .status()
    }

    #[tokio::test]
    async fn the_health_probe_answers_200_and_any_other_path_404() {
        assert_eq!(status(app(), "/healthz").await, StatusCode::OK);
        assert_eq!(status(app(), "/nothing").await, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_request_past_the_deadline_is_ended_with_503() {
        let slow = Router::new().route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(1)).await;
                "late"
            }),
        );
        let app = with_deadline(slow, Duration::from_millis(50));
        assert_eq!(status(app, "/slow").await, StatusCode::SERVICE_UNAVAILABLE);
    }
}
