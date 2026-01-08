use std::{env, error::Error};

use diesel::{Connection as _, PgConnection, pg::Pg};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn run_migrations<MH>(conn: &mut MH) -> Result<(), Box<dyn Error + Send + Sync + 'static>>
where
    MH: MigrationHarness<Pg>,
{
    conn.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}
