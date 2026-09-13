#[tokio::main]
async fn main() {
    let url = std::env::var("PHOTARA_TEST_MIGRATOR_URL").expect("explicit disposable URL");
    assert!(
        url.contains("photara_cxt3c") && url.contains("private"),
        "disposable Unix socket URL required"
    );
    photara_service::migrate(storexa::DatabaseConfig::from_url(url).unwrap())
        .await
        .unwrap();
    println!("CXT3c service migrations applied");
}
