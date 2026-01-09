use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};

#[derive(serde::Deserialize, Debug)]
pub struct CSVRecord {
    #[serde(
        rename = "Time (UTC)",
        deserialize_with = "super::deserialize_datetime"
    )]
    pub datetime: DateTime<Utc>,
    #[serde(
        rename = "Quantity kWh",
        deserialize_with = "super::deserialize_decimal"
    )]
    pub amount: BigDecimal,
}
