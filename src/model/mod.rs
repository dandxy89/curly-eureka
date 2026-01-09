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
    use chrono::{DateTime, Utc};
    use diesel::{Insertable, Queryable, QueryableByName};

    #[derive(Queryable, Insertable, QueryableByName, Debug)]
    #[diesel(table_name = crate::schema::ts_metadata)]
    pub struct TSMetadata {
        pub ingestion_id: i64,
        pub ingestion_datetime: DateTime<Utc>,
        pub source: String,
    }
}

pub mod request {
    use chrono::{DateTime, Utc};
    use diesel::{AsExpression, deserialize::FromSqlRow};

    #[derive(Debug, PartialEq, Eq, FromSqlRow, AsExpression)]
    #[diesel(sql_type = crate::schema::sql_types::AggregationKind)]
    pub enum AggregationKind {
        Hourly,
        DayInMonth,
        Monthly,
        Yearly,
    }

    #[derive(Debug)]
    pub struct TimeSeriesRange {
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
    }
}

pub mod response {
    use bigdecimal::BigDecimal;
    use chrono::{DateTime, Utc};

    use crate::model::request::{AggregationKind, TimeSeriesRange};

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

    #[derive(Debug)]
    pub struct QueryHistoryRecord {
        executed_at: DateTime<Utc>,
        time_range: TimeSeriesRange,
        aggregation: AggregationKind,
    }
}
