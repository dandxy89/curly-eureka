#![allow(dead_code)]

pub mod api_request;
pub mod api_response;
pub mod csv;
pub mod database;

use std::str::FromStr;

use bigdecimal::{BigDecimal, ToPrimitive as _};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize as _, Deserializer, Serializer, de::Error};

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

pub fn serialize_opt_bigdecimal<S>(
    value: &Option<BigDecimal>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_some(&v.to_f64()),
        None => serializer.serialize_none(),
    }
}
