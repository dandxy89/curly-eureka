// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "aggregation_kind"))]
    pub struct AggregationKind;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AggregationKind;

    query_history (id) {
        id -> Int8,
        executed_at -> Timestamptz,
        from_date -> Nullable<Timestamptz>,
        to_date -> Nullable<Timestamptz>,
        aggregation -> AggregationKind,
    }
}

diesel::table! {
    ts_metadata (ingestion_id) {
        ingestion_id -> Int8,
        ingestion_datetime -> Timestamptz,
        source -> Text,
    }
}

diesel::table! {
    ts_store (ingestion_id, datetime) {
        ingestion_id -> Int8,
        datetime -> Timestamptz,
        amount -> Numeric,
    }
}

diesel::joinable!(ts_store -> ts_metadata (ingestion_id));

diesel::allow_tables_to_appear_in_same_query!(query_history, ts_metadata, ts_store,);
