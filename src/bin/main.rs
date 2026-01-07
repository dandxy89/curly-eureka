use std::time::Duration;

use axum::{Router, http::StatusCode, routing::get};
use renewable_ts_axum::{logger::init_logging, shutdown::shutdown_signal};
use tokio::net::TcpListener;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::info;

#[tokio::main]
async fn main() {
    init_logging();

    // TODO: Config / EnvVars?
    // TODO: Migrations
    // TODO: Truncate or Handle Conflicts (on load of CSV)

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        // TODO: .route()  - History Endpoint
        // TODO: .route()  - Query Endpoint
        .layer((
            TraceLayer::new_for_http(),
            TimeoutLayer::with_status_code(StatusCode::GATEWAY_TIMEOUT, Duration::from_secs(10)),
        ));
    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();

    info!("Starting Axum Server...");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
