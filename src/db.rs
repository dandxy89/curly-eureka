use std::{env, fs::File, io::BufReader, path::Path};

use deadpool_diesel::{InteractError, Manager, Pool, PoolError, Runtime, postgres::BuildError};
use diesel::{OptionalEmptyChangesetExtension, PgConnection, RunQueryDsl, connection::Connection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness as _, embed_migrations};
use tracing::{error, info};

use crate::{
    file_reader::csv_stream,
    model::database::{TSMetadata, TSStore},
    renewable_schema,
};

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
            let Ok(Some(ingestion_id)) = diesel::insert_into(renewable_schema::ts_metadata::table)
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
