#![allow(dead_code)]

mod decoder {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{Deserialize as _, Deserializer, de::Error};

    const DATETIME_FORMAT: &str = "%-d %b %Y %H:%M";

    pub fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let naive = NaiveDateTime::parse_from_str(&s, DATETIME_FORMAT).map_err(D::Error::custom)?;
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
    }

    pub fn deserialize_decimal<'de, D>(deserializer: D) -> Result<BigDecimal, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).and_then(|s| {
            let cleaned_string = s.trim().trim_matches('"').replace(',', "");
            if cleaned_string.is_empty() {
                return Err(Error::custom(
                    "Unable to parse to BigDecimal due to empty string",
                ));
            }

            BigDecimal::from_str(&cleaned_string)
                .map_err(|err| Error::custom(format!("Unable to parse to BigDecimal: {err}")))
        })
    }
}

pub mod csv {
    use bigdecimal::BigDecimal;
    use chrono::{DateTime, Utc};

    #[derive(serde::Deserialize, Debug)]
    pub struct CSVRecord {
        #[serde(
            rename = "Time (UTC)",
            deserialize_with = "super::decoder::deserialize_datetime"
        )]
        pub datetime: DateTime<Utc>,
        #[serde(
            rename = "Quantity kWh",
            deserialize_with = "super::decoder::deserialize_decimal"
        )]
        pub amount: BigDecimal,
    }
}

pub mod database {
    use bigdecimal::BigDecimal;
    use chrono::{DateTime, Utc};
    use diesel::{Insertable, Queryable, QueryableByName, Selectable};
    use serde::Serialize;

    use crate::model::{csv::CSVRecord, request::AggregationKind};

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
        pub id: i64,
        pub executed_at: DateTime<Utc>,
        pub from_date: Option<DateTime<Utc>>,
        pub to_date: Option<DateTime<Utc>>,
        pub aggregation: AggregationKind,
    }
}

pub mod request {
    use chrono::{DateTime, Utc};
    use diesel::{
        AsExpression,
        deserialize::{FromSql, FromSqlRow},
        pg::{Pg, PgValue},
    };
    use serde::{Deserialize, Serialize};

    #[derive(
        Debug, PartialEq, Eq, FromSqlRow, AsExpression, Deserialize, Serialize, Clone, Copy,
    )]
    #[diesel(sql_type = crate::renewable_schema::sql_types::AggregationKind)]
    pub enum AggregationKind {
        Hourly,
        DayInMonth,
        Monthly,
        Yearly,
    }

    impl FromSql<crate::renewable_schema::sql_types::AggregationKind, Pg> for AggregationKind {
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

    #[derive(Debug, Deserialize)]
    pub struct TimeSeriesRange {
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
    }

    #[derive(Debug, Deserialize)]
    pub struct TimeSeriesAggregationRequest {
        aggregation_kind: AggregationKind,
        datetime_filter: TimeSeriesRange,
    }
}

pub mod response {
    use bigdecimal::BigDecimal;
    use chrono::{DateTime, Utc};

    #[derive(Debug)]
    pub struct AggregationQueryRecord {
        datetime: DateTime<Utc>,
        total_amount: BigDecimal,
    }

    #[derive(Debug)]
    pub struct QueryResponse {
        executed_at: DateTime<Utc>,
        records: Vec<AggregationQueryRecord>,
    }
}
