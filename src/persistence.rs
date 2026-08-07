use storexa::{Database, DatabaseConfig, PostgresProvider};

const DEVELOPMENT_DATABASE_URL: &str = "PHOTARA_DEV_DATABASE_URL";

pub async fn connect_development() -> storexa::Result<Database> {
    let config = DatabaseConfig::from_env_var(DEVELOPMENT_DATABASE_URL)?
        .with_name("photara-development")?
        .with_provider(PostgresProvider::Neon)?
        .with_max_connections(5);

    Database::connect(config).await
}
