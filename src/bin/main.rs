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
    shutdown::shutdown_signal,
};
use tokio::net::TcpListener;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{error, info};

mod routing {
    use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
    use chrono::Utc;
    use deadpool_diesel::postgres::Pool;
    use renewable_ts_axum::db::query::aggregate_ts_query;
    use renewable_ts_axum::{
        db::query::query_request_history,
        model::{
            request::{TimeSeriesAggregationRequest, TimeSeriesRange},
            response::QueryResponse,
        },
    };
    use serde_json::json;
    use tracing::{error, info};

    pub async fn handler_404() -> impl IntoResponse {
        (StatusCode::NOT_FOUND, "")
    }

    pub async fn post_query_ts(
        State(pg_pool): State<Pool>,
        Json(request): Json<TimeSeriesAggregationRequest>,
    ) -> impl IntoResponse {
        let Ok(conn) = pg_pool.get().await else {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response();
        };

        let TimeSeriesAggregationRequest {
            aggregation_kind,
            datetime_filter: TimeSeriesRange { from_date, to_date },
        } = request;
        info!(aggregation_kind= ?aggregation_kind, from_date= ?from_date, to_date= ?to_date, "Received Time Series Query");
        let query_result = conn
            .interact(move |conn| aggregate_ts_query(aggregation_kind, from_date, to_date, conn))
            .await;

        let Ok(query_result) = query_result else {
            error!("Error executing aggregate query");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response();
        };

        match query_result {
            Ok(records) => {
                let response = QueryResponse {
                    executed_at: Utc::now(),
                    records,
                };
                Json(response).into_response()
            }
            Err(e) => {
                error!("Error executing aggregate query: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response()
            }
        }
    }

    pub async fn get_query_history(State(pg_pool): State<Pool>) -> impl IntoResponse {
        let Ok(conn) = pg_pool.get().await else {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response();
        };

        let Ok(history_result) = conn.interact(query_request_history).await else {
            error!("Error executing Query History");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response();
        };

        match history_result {
            Ok(records) => Json(json!(records)).into_response(),
            Err(e) => {
                error!("Error executing Query History: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response()
            }
        }
    }
}

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
        .route("/timeseries/v1/query", post(routing::post_query_ts))
        // Query History Endpoint
        .route(
            "/timeseries/v1/query/history",
            get(routing::get_query_history),
        )
        .fallback(routing::handler_404)
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
