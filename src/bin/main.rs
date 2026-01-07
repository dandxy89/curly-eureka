use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[tokio::main]
async fn main() {
    // Initialise Logging using the Tracing crate
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
