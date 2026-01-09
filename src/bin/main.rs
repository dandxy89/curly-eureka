use std::{error::Error, net::SocketAddr, time::Duration};

use axum::{Router, http::StatusCode, routing::get};
use dotenvy::dotenv;
use renewable_ts_axum::{
    db::{establish_pg_connection, seed_database},
    logger::init_logging,
    shutdown::shutdown_signal,
};
use tokio::net::TcpListener;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    init_logging();

    let pg_pool = establish_pg_connection()
        .await
        .inspect_err(|e| error!("Unable to configure DB: {e:?}"))?;

    seed_database(&pg_pool).await?;

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        // TODO: .route()  - History Endpoint
        // TODO: .route()  - Query Endpoint
        .layer((
            TraceLayer::new_for_http(),
            TimeoutLayer::with_status_code(StatusCode::GATEWAY_TIMEOUT, Duration::from_secs(10)),
        ))
        .with_state(pg_pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("listening on {addr}");
    let listener = TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
