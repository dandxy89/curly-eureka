use std::{error::Error, net::SocketAddr, time::Duration};

use axum::{
    Router,
    http::StatusCode,
    routing::{get, post},
};
use dotenvy::dotenv;
use renewable_ts_axum::{
    db::{establish_pg_connection, seed_database},
    logger::init_logging,
    shutdown::shutdown_signal,
};
use tokio::net::TcpListener;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{error, info};

mod routing {
    use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
    use deadpool_diesel::postgres::Pool;
    use diesel::{ExpressionMethods as _, QueryDsl, RunQueryDsl as _, SelectableHelper as _};
    use renewable_ts_axum::model::{database::QueryHistory, request::TimeSeriesAggregationRequest};
    use serde_json::json;
    use tracing::error;

    const DEFAULT_HISTORY_LIMIT: i64 = 10;

    pub async fn handler_404() -> impl IntoResponse {
        (StatusCode::NOT_FOUND, "")
    }

    pub async fn post_query_ts(
        State(_pg_pool): State<Pool>,
        Json(_request): Json<TimeSeriesAggregationRequest>,
    ) -> impl IntoResponse {
        (StatusCode::INTERNAL_SERVER_ERROR, "")
    }

    pub async fn get_query_history(State(pg_pool): State<Pool>) -> impl IntoResponse {
        use renewable_ts_axum::renewable_schema::query_history::dsl::{executed_at, query_history};

        let Ok(conn) = pg_pool.get().await else {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response();
        };

        let history_result = conn
            .interact(|conn| {
                query_history
                    .select(QueryHistory::as_select())
                    .order_by(executed_at.desc())
                    .limit(DEFAULT_HISTORY_LIMIT)
                    .get_results::<QueryHistory>(conn)
            })
            .await;

        match history_result {
            Ok(Ok(records)) => Json(json!(records)).into_response(),
            Ok(Err(e)) => {
                error!("Error executing Query History: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response()
            }
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

    let pg_pool = establish_pg_connection()
        .await
        .inspect_err(|e| error!("Unable to configure DB: {e:?}"))?;

    seed_database(&pg_pool).await?;

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
