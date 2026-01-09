use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, diesel::Queryable, Serialize)]
pub struct AggregationQueryRecord {
    pub datetime: DateTime<Utc>,
    #[serde(serialize_with = "super::serialize_opt_bigdecimal")]
    pub total_amount: Option<BigDecimal>,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub executed_at: DateTime<Utc>,
    pub records: Vec<AggregationQueryRecord>,
}
