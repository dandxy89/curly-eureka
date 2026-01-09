CREATE TYPE aggregation_kind AS ENUM ('Hourly', 'DayInMonth', 'Monthly', 'Yearly');

CREATE TABLE ts_metadata (
    ingestion_id BIGSERIAL PRIMARY KEY,
    ingestion_datetime TIMESTAMPTZ NOT NULL DEFAULT now(),
    source TEXT NOT NULL
);

CREATE TABLE ts_store (
    ingestion_id BIGINT NOT NULL REFERENCES ts_metadata(ingestion_id),
    datetime TIMESTAMPTZ NOT NULL,
    amount NUMERIC(20, 6) NOT NULL,

    PRIMARY KEY (ingestion_id, datetime)
);

CREATE TABLE query_history (
    id BIGSERIAL PRIMARY KEY,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    from_date TIMESTAMPTZ,
    to_date TIMESTAMPTZ,
    aggregation aggregation_kind NOT NULL
);

CREATE INDEX idx_ts_store_datetime ON ts_store(ingestion_id, datetime);
CREATE INDEX idx_query_history_executed_at ON query_history(executed_at);
