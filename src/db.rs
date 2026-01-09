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
    MigrationError(InteractError),
    #[error("unable to get connection from pool")]
    ConnectionError(PoolError),
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
            .map_err(PgError::MigrationError)?
            .unwrap();
    }

    Ok(pg_pool)
}
