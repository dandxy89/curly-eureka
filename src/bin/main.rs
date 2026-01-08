use std::{net::SocketAddr, time::Duration};

use axum::{Router, http::StatusCode, routing::get};
use dotenvy::dotenv;
use renewable_ts_axum::{
    db::{establish_connection, run_migrations},
    logger::init_logging,
    shutdown::shutdown_signal,
};
use tokio::net::TcpListener;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::info;

#[tokio::main]
async fn main() {
    dotenv().ok();
    init_logging();

    let mut pg_connection = establish_connection();
    run_migrations(&mut pg_connection).unwrap();

    // TODO: Truncate or Handle Conflicts (on load of CSV)

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        // TODO: .route()  - History Endpoint
        // TODO: .route()  - Query Endpoint
        .layer((
            TraceLayer::new_for_http(),
            TimeoutLayer::with_status_code(StatusCode::GATEWAY_TIMEOUT, Duration::from_secs(10)),
        ));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("listening on {addr}");
    let listener = TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
