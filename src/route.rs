use crate::{
    db::query::{aggregate_ts_query, query_request_history},
    model::{
        api_request::{TimeSeriesAggregationRequest, TimeSeriesRange},
        api_response::QueryResponse,
    },
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use deadpool_diesel::postgres::Pool;
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
