# Technical Test for Renewable

## Setup Locally

Follow the steps below to get setup:

```bash
# Install the diesel_cli
# https://github.com/diesel-rs/diesel/blob/main/diesel_cli/README.md
cargo install diesel_cli --no-default-features --features "postgres"

# Start a local PG instance
docker compose up -d --force-recreate

# Setup with Migrations
echo $DATABASE_URL
diesel database setup
diesel database reset
```

## Example Curl Queries

```bash
# Aggregation ONLY
curl -X POST -H "Content-Type: application/json" -d '{"aggregation_kind": "Hourly", "datetime_filter": {}}' 0.0.0.0:8000/timeseries/v1/query | jq
curl -X POST -H "Content-Type: application/json" -d '{"aggregation_kind": "Monthly", "datetime_filter": {}}' 0.0.0.0:8000/timeseries/v1/query | jq
curl -X POST -H "Content-Type: application/json" -d '{"aggregation_kind": "Yearly", "datetime_filter": {}}' 0.0.0.0:8000/timeseries/v1/query | jq

# Aggregation AND date_filtering
curl -X POST -H "Content-Type: application/json" -d '{"aggregation_kind": "Monthly", "datetime_filter": {"from_date": "2025-01-01T00:00:00Z", "to_date": "2025-01-19T00:00:00Z"}}' 0.0.0.0:8000/timeseries/v1/query | jq

# Show query history
curl -X GET 0.0.0.0:8000/timeseries/v1/query/history | jq
```
