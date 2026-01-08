use std::io;

use crate::model::csv::CSVRecord;

pub fn csv_stream<R: io::Read>(buffer: R) -> impl Iterator<Item = Result<CSVRecord, csv::Error>> {
    let reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(buffer);

    reader.into_deserialize::<CSVRecord>()
}

#[cfg(test)]
mod test {
    use bigdecimal::{BigDecimal, FromPrimitive};
    use chrono::DateTime;

    #[test]
    fn test_csv_decoding() {
        let test_data = r#"Time (UTC),Quantity kWh
1 Jan 2025 00:00,"9,000.000"
1 Jan 2025 01:00,"9,000.000"
1 Jan 2025 02:00,"9,000.000"
1 Jan 2025 03:00,"9,000.000"
1 Jan 2025 04:00,"9,000.000"
1 Jan 2025 05:00,"9,000.000"
1 Jan 2025 06:00,"9,000.000"
"#;
        let mut reader = super::csv_stream(test_data.as_bytes());
        let record = reader.next().unwrap().unwrap();

        let expected_dt = DateTime::parse_from_rfc3339("2025-01-01T00:00:00-00:00").unwrap();
        assert_eq!(record.datetime, expected_dt);
        assert_eq!(record.amount, BigDecimal::from_i32(9000).unwrap());

        assert_eq!(reader.count(), 6);
    }
}
