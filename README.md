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

# Copy example env
cp .env.example .env

# Test the Code Base (requires Docker instance running)
cargo test
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

## Deployment

- Deploying the Binary: Create Dockerfile with 2 stages
  1. The first stage should be used for building the container e.g. `rust:1.92.0-slim-bookworm`
  2. The second stahe will then be used for deployment into ECR where the Image is more lightweight e.g. `debian:bookworm-20251117-slim`
     a. Configuration for DB passed in at runtime via the SecretsManager via Environment Variables
     b. Create a non-root user and limit the permissions where possible

### AWS Deployment

- Push image to ECR via GH Workflow
- Deploy to ECS (Fargate launch type)
- Route traffic through an Application Load Balancer (ALB)
  - Consider API Gateway + ALB eventually
