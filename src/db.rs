use std::env;

use deadpool_diesel::{InteractError, Manager, Pool, PoolError, Runtime, postgres::BuildError};
use diesel::PgConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness as _, embed_migrations};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[derive(thiserror::Error, Debug)]
pub enum PgError {
    #[error("missing DATABASE_URL")]
    DatabaseURL,

    #[error("unable to build pool")]
    PoolBuildError(BuildError),

    #[error("unable to apply migrations {0}")]
    InteractionError(InteractError),

    #[error("unable to get connection from pool")]
    ConnectionError(PoolError),

    #[error("missing SEED_FILE")]
    SeedFilePath,

    #[error("invalid SEED_FILE")]
    SeedFileValidationError,

    #[error("unable to seed database")]
    SeedDatabaseError,

    #[error("diesel errorer {0}")]
    DieselError(diesel::result::Error),
}

pub async fn establish_pg_connection() -> Result<Pool<Manager<PgConnection>>, PgError> {
    let database_url = env::var("DATABASE_URL").map_err(|_| PgError::DatabaseURL)?;
    let pg_manager = Manager::new(database_url, Runtime::Tokio1);

    let pg_pool: Pool<Manager<PgConnection>> = Pool::builder(pg_manager)
        .build()
        .map_err(PgError::PoolBuildError)?;

    {
        let conn = pg_pool.get().await.map_err(PgError::ConnectionError)?;

        conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
            .await
            .map_err(PgError::InteractionError)?
            .unwrap();
    }

    Ok(pg_pool)
}

pub mod seed_database {
    use std::{env, fs::File, io::BufReader, path::Path};

    use diesel::{OptionalEmptyChangesetExtension, RunQueryDsl, connection::Connection};
    use tracing::{error, info};

    use crate::{
        db::PgError,
        file_reader::csv_stream,
        model::database::{TSMetadata, TSStore},
        renewable_schema,
    };

    fn get_seed_file(path_str: &str) -> Result<File, PgError> {
        let seed_filepath = Path::new(path_str);
        if !seed_filepath.is_file() {
            error!("SEED_FILE path does not exist");
            return Err(PgError::SeedFileValidationError);
        }
        let Some(extension) = seed_filepath.extension() else {
            error!("SEED_FILE extension not known");
            return Err(PgError::SeedFileValidationError);
        };
        if extension != "csv" {
            error!("SEED_FILE should be a .csv file");
            return Err(PgError::SeedFileValidationError);
        }

        File::open(seed_filepath).map_err(|_| PgError::SeedFileValidationError)
    }

    pub async fn seed_database(pg_pool: &deadpool_diesel::postgres::Pool) -> Result<(), PgError> {
        info!("Seeding database");
        let env_var: String = env::var("SEED_FILE").map_err(|_| PgError::SeedFilePath)?;
        let seed_file = get_seed_file(&env_var)?;

        let conn = pg_pool.get().await.map_err(PgError::ConnectionError)?;

        conn.interact(|conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                // Insert Metadata about the seed file
                let Ok(Some(ingestion_id)) =
                    diesel::insert_into(renewable_schema::ts_metadata::table)
                        .values(TSMetadata::new(env_var))
                        .returning(renewable_schema::ts_metadata::ingestion_id)
                        .on_conflict_do_nothing()
                        .get_result::<i64>(conn)
                        .optional_empty_changeset()
                else {
                    info!("Data has already been ingested");
                    return Ok(());
                };

                // Read in the data from the .csv file
                let buffer = BufReader::new(seed_file);
                let records: Vec<TSStore> = csv_stream(buffer)
                    .flatten()
                    .map(|r| (ingestion_id, r).into())
                    .collect();

                // Insert Time Series data
                let inserted_rows = diesel::insert_into(renewable_schema::ts_store::table)
                    .values(records)
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .optional_empty_changeset()?;

                info!("Seeded database with {inserted_rows:?} records");
                Ok(())
            })
        })
        .await
        .map_err(PgError::InteractionError)?
        .map_err(PgError::DieselError)?;

        Ok(())
    }
}

pub mod query {

    use crate::{
        model::{
            api_request::Aggregation, api_response::AggregationQueryRecord, database::QueryHistory,
        },
        renewable_schema::{
            query_history::dsl::{executed_at, query_history},
            ts_store,
        },
    };
    use chrono::Utc;
    use diesel::Connection as _;
    use diesel::dsl::sql;
    use diesel::sql_types::{Nullable, Numeric};
    use diesel::{
        ExpressionMethods as _, QueryDsl as _, RunQueryDsl as _, SelectableHelper as _,
        define_sql_function,
        sql_types::{Text, Timestamptz},
    };

    pub(crate) const DEFAULT_HISTORY_LIMIT: i64 = 10;

    define_sql_function! {
        #[sql_name = "DATE_TRUNC"]
        fn date_trunc(period: Text, ts: Timestamptz) -> Timestamptz;
    }

    pub fn query_request_history(
        conn: &mut diesel::PgConnection,
    ) -> Result<Vec<QueryHistory>, diesel::result::Error> {
        query_history
            .select(QueryHistory::as_select())
            .order_by(executed_at.desc())
            .limit(DEFAULT_HISTORY_LIMIT)
            .get_results::<QueryHistory>(conn)
    }

    pub fn aggregate_ts_query(
        aggregation_kind: Aggregation,
        from_date: Option<chrono::DateTime<Utc>>,
        to_date: Option<chrono::DateTime<Utc>>,
        conn: &mut diesel::PgConnection,
    ) -> Result<Vec<AggregationQueryRecord>, diesel::result::Error> {
        conn.transaction(|conn| {
            // Persist the query in history
            let history_entry = QueryHistory::new(from_date, to_date, aggregation_kind);
            diesel::insert_into(query_history)
                .values(&history_entry)
                .execute(conn)?;

            // Construct and execute the aggregation query
            let period = <&str>::from(aggregation_kind);
            let datetime_expr = sql::<Timestamptz>(&format!("DATE_TRUNC('{period}', datetime)"));
            let sum_expr = sql::<Nullable<Numeric>>("SUM(amount)");
            let group_expr = sql::<Timestamptz>(&format!("DATE_TRUNC('{period}', datetime)"));

            let mut query = ts_store::table
                .select((datetime_expr, sum_expr))
                .group_by(group_expr)
                .into_boxed();

            if let Some(from) = from_date {
                query = query.filter(ts_store::datetime.ge(from));
            }
            if let Some(to) = to_date {
                query = query.filter(ts_store::datetime.le(to));
            }

            query.load(conn)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use bigdecimal::BigDecimal;
    use chrono::{DateTime, Duration, TimeZone, Utc};
    use diesel::{Connection, PgConnection, RunQueryDsl};
    use serial_test::serial;
    use test_case::test_case;

    use crate::{
        db::query::{DEFAULT_HISTORY_LIMIT, aggregate_ts_query, query_request_history},
        model::{api_request::Aggregation, database::TSStore},
        renewable_schema::{query_history, ts_metadata, ts_store},
    };

    fn get_test_connection() -> PgConnection {
        dotenvy::dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        PgConnection::establish(&database_url).expect("Failed to connect to database")
    }

    fn cleanup_tables(conn: &mut PgConnection) {
        diesel::delete(query_history::table).execute(conn).unwrap();
        diesel::delete(ts_store::table).execute(conn).unwrap();
        diesel::delete(ts_metadata::table).execute(conn).unwrap();
    }

    fn seed_ts_metadata(conn: &mut PgConnection) -> i64 {
        use crate::model::database::TSMetadata;

        diesel::insert_into(ts_metadata::table)
            .values(TSMetadata::new("test_source".to_string()))
            .returning(ts_metadata::ingestion_id)
            .get_result::<i64>(conn)
            .unwrap()
    }

    fn seed_ts_data(conn: &mut PgConnection, ingestion_id: i64) {
        let base_date = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        let records: Vec<TSStore> = (0..48)
            .map(|i| TSStore {
                ingestion_id,
                datetime: base_date + Duration::hours(i),
                amount: BigDecimal::from(100 * (i + 1)),
            })
            .collect();

        diesel::insert_into(ts_store::table)
            .values(&records)
            .execute(conn)
            .expect("Failed to seed ts_store");
    }

    fn test_from_date() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap()
    }

    fn test_to_date() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 16, 12, 0, 0).unwrap()
    }

    #[test]
    #[serial]
    fn test_query_request_history_respects_limit_and_ordering() {
        let mut conn = get_test_connection();
        cleanup_tables(&mut conn);

        let ingestion_id = seed_ts_metadata(&mut conn);
        seed_ts_data(&mut conn, ingestion_id);

        for _ in 0..15 {
            aggregate_ts_query(Aggregation::Hourly, None, None, &mut conn).unwrap();
        }

        let result = query_request_history(&mut conn);
        assert!(result.is_ok());
        let history = result.unwrap();
        assert_eq!(history.len(), DEFAULT_HISTORY_LIMIT as usize);

        for i in 0..history.len() - 1 {
            assert!(history[i].executed_at >= history[i + 1].executed_at);
        }
    }

    #[test_case(Aggregation::Hourly, None, None)]
    #[test_case(Aggregation::Hourly, Some(test_from_date()), None)]
    #[test_case(Aggregation::Hourly, None, Some(test_to_date()))]
    #[test_case(Aggregation::Hourly, Some(test_from_date()), Some(test_to_date()))]
    #[test_case(Aggregation::DayInMonth, None, None)]
    #[test_case(Aggregation::DayInMonth, Some(test_from_date()), None)]
    #[test_case(Aggregation::DayInMonth, None, Some(test_to_date()))]
    #[test_case(Aggregation::DayInMonth, Some(test_from_date()), Some(test_to_date()))]
    #[test_case(Aggregation::Monthly, None, None)]
    #[test_case(Aggregation::Monthly, Some(test_from_date()), None)]
    #[test_case(Aggregation::Monthly, None, Some(test_to_date()))]
    #[test_case(Aggregation::Monthly, Some(test_from_date()), Some(test_to_date()))]
    #[test_case(Aggregation::Yearly, None, None)]
    #[test_case(Aggregation::Yearly, Some(test_from_date()), None)]
    #[test_case(Aggregation::Yearly, None, Some(test_to_date()))]
    #[test_case(Aggregation::Yearly, Some(test_from_date()), Some(test_to_date()))]
    #[serial]
    fn test_aggregate_ts_query(
        aggregation_kind: Aggregation,
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
    ) {
        let mut conn = get_test_connection();
        cleanup_tables(&mut conn);

        let ingestion_id = seed_ts_metadata(&mut conn);
        seed_ts_data(&mut conn, ingestion_id);

        let result = aggregate_ts_query(aggregation_kind, from_date, to_date, &mut conn);
        assert!(result.is_ok());
        let records = result.unwrap();

        let history = query_request_history(&mut conn).unwrap();
        assert!(!history.is_empty());

        let latest_entry = &history[0];
        assert_eq!(latest_entry.aggregation, aggregation_kind);
        assert_eq!(latest_entry.from_date, from_date);
        assert_eq!(latest_entry.to_date, to_date);

        if from_date.is_some() || to_date.is_some() {
            let unfiltered = aggregate_ts_query(aggregation_kind, None, None, &mut conn).unwrap();
            assert!(records.len() <= unfiltered.len());
        }
    }
}
