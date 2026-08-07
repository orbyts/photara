mod persistence;

use std::{env, error::Error};

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().with_target(false).try_init().ok();

    match env::args().nth(1).as_deref() {
        None => println!("Hello from Photara."),
        Some("health") => health().await?,
        Some(command) => return Err(format!("unknown command: {command}").into()),
    }

    Ok(())
}

async fn health() -> storexa::Result<()> {
    let database = persistence::connect_development().await?;
    let report = database.health().await?;

    info!(
        provider = %database.provider(),
        server_version = %report.server_version,
        latency_ms = report.latency.as_millis(),
        "Photara database is healthy"
    );

    database.close().await;
    Ok(())
}
