use axum::{Router, routing::get};
use renewable_ts_axum::{logger::init_logging, shutdown::shutdown_signal};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialise Logging using the Tracing crate
    init_logging();

    let app = Router::new().route("/", get(|| async { "Hello, World!" }));
    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();

    info!("Starting Axum Server...");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
