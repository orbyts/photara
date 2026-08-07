use storexa::{Database, DatabaseConfig, MigrationReport, PostgresProvider};

use crate::Result;

const DEVELOPMENT_DATABASE_URL: &str = "PHOTARA_DEV_DATABASE_URL";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn connect_development() -> Result<Database> {
    let config = DatabaseConfig::from_env_var(DEVELOPMENT_DATABASE_URL)?
        .with_name("photara-development")?
        .with_provider(PostgresProvider::Neon)?
        .with_max_connections(5);

    Ok(Database::connect(config).await?)
}

pub async fn migrate(database: &Database) -> Result<MigrationReport> {
    Ok(database.run_migrations(&MIGRATOR).await?)
}
