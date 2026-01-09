use chrono::{DateTime, Utc};
use diesel::{
    AsExpression,
    deserialize::{FromSql, FromSqlRow},
    pg::{Pg, PgValue},
    serialize::{IsNull, Output, ToSql},
};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Debug, PartialEq, Eq, FromSqlRow, AsExpression, Deserialize, Serialize, Clone, Copy)]
#[diesel(sql_type = crate::renewable_schema::sql_types::AggregationKind)]
pub enum Aggregation {
    Hourly,
    DayInMonth,
    Monthly,
    Yearly,
}

impl FromSql<crate::renewable_schema::sql_types::AggregationKind, Pg> for Aggregation {
    fn from_sql(bytes: PgValue<'_>) -> diesel::deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"Hourly" => Ok(Self::Hourly),
            b"DayInMonth" => Ok(Self::DayInMonth),
            b"Monthly" => Ok(Self::Monthly),
            b"Yearly" => Ok(Self::Yearly),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<crate::renewable_schema::sql_types::AggregationKind, Pg> for Aggregation {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        let s = match self {
            Self::Hourly => "Hourly",
            Self::DayInMonth => "DayInMonth",
            Self::Monthly => "Monthly",
            Self::Yearly => "Yearly",
        };
        out.write_all(s.as_bytes())?;
        Ok(IsNull::No)
    }
}

impl From<Aggregation> for &str {
    fn from(kind: Aggregation) -> Self {
        match kind {
            Aggregation::Hourly => "hour",
            Aggregation::DayInMonth => "day",
            Aggregation::Monthly => "month",
            Aggregation::Yearly => "year",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TimeSeriesRange {
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct TimeSeriesAggregationRequest {
    pub aggregation_kind: Aggregation,
    pub datetime_filter: TimeSeriesRange,
}
