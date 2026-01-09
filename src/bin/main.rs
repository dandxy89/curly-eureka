use std::{error::Error, net::SocketAddr, time::Duration};

use axum::{
    Router,
    http::StatusCode,
    routing::{get, post},
};
use dotenvy::dotenv;
use renewable_ts_axum::{
    db::{establish_pg_connection, seed_database::seed_database},
    logger::init_logging,
    route,
    shutdown::shutdown_signal,
};
use tokio::net::TcpListener;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    init_logging();

    // Create Postgres connection pool and run migrations
    let pg_pool = establish_pg_connection()
        .await
        .inspect_err(|e| error!("Unable to configure DB: {e:?}"))?;

    // Seed the database with initial data
    seed_database(&pg_pool).await?;

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("listening on {addr}");
    let listener = TcpListener::bind(addr).await.unwrap();

    let app = Router::new()
        // Query Endpoint
        .route("/timeseries/v1/query", post(route::post_query_ts))
        // Query History Endpoint
        .route(
            "/timeseries/v1/query/history",
            get(route::get_query_history),
        )
        .fallback(route::handler_404)
        .layer((
            TraceLayer::new_for_http(),
            TimeoutLayer::with_status_code(StatusCode::GATEWAY_TIMEOUT, Duration::from_secs(2)),
        ))
        .with_state(pg_pool);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
