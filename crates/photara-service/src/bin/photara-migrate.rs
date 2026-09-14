use std::env;
use storexa::DatabaseConfig;

#[tokio::main]
async fn main() {
    let Some(url) = env::var("PHOTARA_DB_MIGRATION_URL")
        .ok()
        .filter(|value| !value.is_empty())
    else {
        eprintln!("photara-migrate:missing-required-setting:PHOTARA_DB_MIGRATION_URL");
        std::process::exit(1);
    };
    let result = async {
        let config = DatabaseConfig::from_url(url).map_err(|_| ())?;
        photara_service::migrate(config).await.map_err(|_| ())
    }
    .await;
    if result.is_err() {
        eprintln!("photara-migrate:migration-refused");
        std::process::exit(1);
    }
}
