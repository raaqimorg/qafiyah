pub mod accounts;
pub mod auth;
pub mod cache;
pub mod client_ip;
pub mod config;
pub mod constants;
pub mod contract;
pub mod cors;
pub mod db;
pub mod domain;
pub mod envelope;
pub mod error;
pub mod es;
pub mod extract;
pub mod js;
pub mod log;
pub mod openapi;
pub mod query;
pub mod rate_limit;
pub mod routes;
pub mod sentry;
pub mod slug;
pub mod state;

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod http_tests;

use std::future::Future;
use std::net::SocketAddr;

use axum::Router;
use axum::extract::OriginalUri;
use axum::http::{Method, Uri};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::get;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::error::{AppError, RouteProblem};

use crate::state::AppState;

pub const API_V1_PREFIX: &str = "/v1";

fn build_contract() -> (Router<AppState>, utoipa::openapi::OpenApi) {
    OpenApiRouter::with_openapi(<openapi::ApiDoc as utoipa::OpenApi>::openapi())
        .routes(routes!(routes::taxonomy::list_meters))
        .routes(routes!(routes::taxonomy::get_meter))
        .routes(routes!(routes::taxonomy::list_rhymes))
        .routes(routes!(routes::taxonomy::get_rhyme))
        .routes(routes!(routes::taxonomy::list_eras))
        .routes(routes!(routes::taxonomy::get_era))
        .routes(routes!(routes::taxonomy::list_themes))
        .routes(routes!(routes::taxonomy::get_theme))
        .routes(routes!(routes::taxonomy::list_collections))
        .routes(routes!(routes::taxonomy::get_collection))
        .routes(routes!(routes::taxonomy::list_poem_types))
        .routes(routes!(routes::taxonomy::get_poem_type))
        .routes(routes!(routes::poems::list))
        .routes(routes!(routes::poems::list_slugs))
        .routes(routes!(routes::poems::count))
        .routes(routes!(routes::poems::facet_counts))
        .routes(routes!(routes::poems::detail))
        .routes(routes!(routes::poets::list))
        .routes(routes!(routes::poets::list_slugs))
        .routes(routes!(routes::poets::detail))
        .routes(routes!(routes::search::search))
        .split_for_parts()
}

fn contract() -> (Router<AppState>, utoipa::openapi::OpenApi) {
    let (router, mut doc) = build_contract();
    openapi::finish(&mut doc);
    (router, doc)
}

pub fn document() -> utoipa::openapi::OpenApi {
    contract().1
}

pub fn app(state: AppState) -> Router {
    let contract = contract().0;

    let cached = contract
        .merge(routes::spec::router())
        .layer(from_fn(cache::layer));

    let uncached = Router::new()
        .merge(routes::poems::uncached_router())
        .merge(routes::go::router())
        .route("/", get(|| async { routes::site::docs_redirect() }));

    let limited = cached
        .merge(uncached)
        .method_not_allowed_fallback(method_not_allowed)
        .layer(from_fn_with_state(state.clone(), rate_limit::layer));

    Router::new()
        .nest(API_V1_PREFIX, limited)
        .nest(
            constants::ACCOUNT_PREFIX,
            routes::account::wrap(routes::account::router(), state.clone()),
        )
        .merge(routes::site::router())
        .fallback(|method: Method, uri: Uri| async move {
            AppError::from(RouteProblem::no_route(&method, uri.path()))
        })
        .layer(from_fn(error::layer))
        .layer(from_fn(log::layer))
        .layer(from_fn(cors::layer))
        .with_state(state)
}

#[expect(
    clippy::print_stdout,
    reason = "the drain notice is one structured line on stdout"
)]
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
    println!("{}", log::stage_event("draining", None));
}

async fn method_not_allowed(method: Method, uri: OriginalUri) -> AppError {
    AppError::from(RouteProblem::method_not_allowed(&method, uri.path()))
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
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use axum::Router;
    use axum::extract::ConnectInfo;
    use axum::routing::get;
    use tokio::sync::{Notify, oneshot};

    #[tokio::test]
    async fn shutdown_drains_an_in_flight_request() {
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let app = Router::new().route("/slow", {
            let entered = entered.clone();
            let release = release.clone();
            get(move || {
                let entered = entered.clone();
                let release = release.clone();
                async move {
                    entered.notify_one();
                    release.notified().await;
                    "done"
                }
            })
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        let (signal, on_signal) = oneshot::channel::<()>();
        let server = tokio::spawn(super::serve(listener, app, async move {
            let _result = on_signal.await;
        }));

        let request =
            tokio::spawn(async move { reqwest::get(format!("http://{address}/slow")).await });
        entered.notified().await;

        signal.send(()).expect("the server is still listening");
        tokio::time::sleep(Duration::from_millis(50)).await;
        release.notify_one();

        let response = tokio::time::timeout(Duration::from_secs(5), request)
            .await
            .expect("an in-flight request must not be cut by the signal")
            .expect("the request task")
            .expect("a response");
        assert_eq!(response.status(), 200);

        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("the server must return once it has drained")
            .expect("the server task")
            .expect("a clean shutdown");
    }

    #[tokio::test]
    async fn shutdown_closes_an_idle_keep_alive_connection() {
        let app = Router::new().route("/ping", get(|| async { "pong" }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        let (signal, on_signal) = oneshot::channel::<()>();
        let server = tokio::spawn(super::serve(listener, app, async move {
            let _result = on_signal.await;
        }));

        let client = reqwest::Client::new();
        let body = client
            .get(format!("http://{address}/ping"))
            .send()
            .await
            .expect("a response")
            .text()
            .await
            .expect("a body");
        assert_eq!(body, "pong");

        signal.send(()).expect("the server is still listening");
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("an idle keep-alive connection must not hold the drain open")
            .expect("the server task")
            .expect("a clean shutdown");
        drop(client);
    }

    #[tokio::test]
    async fn serve_hands_each_request_its_peer_address() {
        let app = Router::new().route(
            "/peer",
            get(|ConnectInfo(peer): ConnectInfo<SocketAddr>| async move { peer.ip().to_string() }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        let server = tokio::spawn(super::serve(listener, app, std::future::pending()));

        let response = reqwest::get(format!("http://{address}/peer"))
            .await
            .expect("a response");
        assert_eq!(response.status(), 200);
        assert_eq!(response.text().await.expect("a body"), "127.0.0.1");
        server.abort();
    }

    #[tokio::test]
    async fn the_assembled_router_answers_the_health_probe() {
        let es = crate::test_support::FakeEs::serving(
            axum::http::StatusCode::OK,
            serde_json::json!({ "hits": { "total": { "value": 0 }, "hits": [] } }),
        )
        .await;
        let app = super::app(crate::test_support::state(&es));
        let sent =
            crate::test_support::send(app, crate::test_support::request("GET", "/healthz")).await;
        assert_eq!(sent.status, axum::http::StatusCode::OK);
        assert_eq!(sent.body, "ok");
        assert_eq!(sent.headers["cache-control"], "no-store");
    }
}
