use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, QueryableByName, Selectable};
use serde::Serialize;

use crate::model::{api_request::AggregationKind, csv::CSVRecord};

#[derive(Queryable, Insertable, QueryableByName, Debug)]
#[diesel(table_name = crate::renewable_schema::ts_metadata)]
pub struct TSMetadata {
    pub ingestion_datetime: DateTime<Utc>,
    pub source: String,
}

impl TSMetadata {
    pub fn new(source: String) -> Self {
        Self {
            ingestion_datetime: Utc::now(),
            source,
        }
    }
}

#[derive(Queryable, Insertable, QueryableByName, Debug)]
#[diesel(table_name = crate::renewable_schema::ts_store)]
pub struct TSStore {
    pub ingestion_id: i64,
    pub datetime: DateTime<Utc>,
    pub amount: BigDecimal,
}

impl From<(i64, CSVRecord)> for TSStore {
    fn from((ingestion_id, CSVRecord { datetime, amount }): (i64, CSVRecord)) -> Self {
        Self {
            ingestion_id,
            datetime,
            amount,
        }
    }
}

#[derive(Queryable, Insertable, QueryableByName, Debug, Selectable, Serialize)]
#[diesel(table_name = crate::renewable_schema::query_history)]
pub struct QueryHistory {
    #[diesel(skip_insertion)]
    pub id: i64,
    pub executed_at: DateTime<Utc>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub aggregation: AggregationKind,
}

impl QueryHistory {
    pub fn new(
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
        aggregation: AggregationKind,
    ) -> Self {
        Self {
            id: 0,
            executed_at: Utc::now(),
            from_date,
            to_date,
            aggregation,
        }
    }
}
