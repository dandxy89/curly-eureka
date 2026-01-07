# Technical Test for Renewable

## Task Definition

1. Create a web backend which allows querying the time series data from the attached file.
    The service must support summing the time series based on different types of aggregation: hourly, day of month, monthly.
    It should also allow setting date filters for the query.
2. Add support for getting the last 10 historic queries with the filter values(dates and type of aggregation).

## Requirements
- The service must be created using `axum-rs` and `diesel-rs`.
- The APIs must respond with `JSON`.
- Data storage in SQL database.
- We expect to see production-like code.
    Information how the service is set up, tested and deployed must be provided.

## Proposed Solution

- REQUESTS: Add Rust Models for the following
    - `TimeSeriesRange`
        - `FromDate` - Optional / ISO 8601
        - `ToDate` - Optional / ISO 8601
    - `TimeSeriesAggregation` Enum
        - Hourly
        - Day in Month
        - Monthly
        - Yearly
    - Will assume all datetimes will be provided in UTC
- ENDPOINTS:
    - `/api/time-series/v1/query`
        - POST
        - Aggregation will all be handled in SQL
        - Either use SQLite or PG
        - Order in ASC order
    - `/api/time-series/v1/history`
        - GET
    - Status Codes
        - `400` invalid payload
            - Will add basic validation to ensure that the From and To datetimes are in the correct format.
        - `500` internal server error
- RESPONSE:
    - `/api/time-series/v1/query`
        - `{"query_executed_ts": "XYZ", "unit": "MwH", "values": [{"datetime": "XYZ": "value": 123}]}`
    - `/api/time-series/v1/history`
        - `[{"query_executed_ts": "XYZ", "time_range": {"from_date": "XYZ"}, "aggregation": "monthly"}]`
- CSV Parsing
    - Add Model that Parses the CSV file
    - On start of the application seed the database (truncate / run migrations)
- Dependencies
    - `serde` & `serde JSON`
    - `axum-rs`
    - `diesel-rs` for interaction with the database
    - `csv` for reading and parsing the CSV file

