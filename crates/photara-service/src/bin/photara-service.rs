use base64::Engine as _;
use photara_service::{
    auth::Clock,
    http::HttpService,
    release::{self, ReleaseChannel, ReleaseEnvironment},
};
use std::{env, net::SocketAddr, sync::Arc, time::SystemTime};

struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        SystemTime::UNIX_EPOCH
            .elapsed()
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .unwrap_or(i64::MAX)
    }
}

fn required(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing-required-setting:{name}"))
}

fn cursor_key() -> Result<[u8; 32], String> {
    let encoded = required("PHOTARA_CURSOR_KEY_B64")?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| "invalid-setting:PHOTARA_CURSOR_KEY_B64".to_owned())?;
    bytes
        .try_into()
        .map_err(|_| "invalid-setting:PHOTARA_CURSOR_KEY_B64".to_owned())
}

fn profile(
    get: impl Fn(&str) -> Option<String>,
) -> Result<(ReleaseEnvironment, SocketAddr), String> {
    if get("PHOTARA_DB_MIGRATION_URL").is_some() {
        return Err("migration-owner-setting-forbidden".into());
    }
    let channel = ReleaseChannel::parse(
        &get("PHOTARA_RELEASE_CHANNEL").unwrap_or_else(|| "production".into()),
    )
    .map_err(|_| "invalid-release-channel".to_owned())?;
    let (_, environment) = release::checked(channel)
        .map_err(|_| "unavailable-or-invalid-release-profile".to_owned())?;
    for (name, expected) in [
        ("PHOTARA_ENVIRONMENT_ID", &environment.environment_id),
        ("PHOTARA_SERVICE_ORIGIN", &environment.api_origin),
        ("PHOTARA_AUTH0_ISSUER", &environment.auth0_issuer),
        ("PHOTARA_AUTH0_AUDIENCE", &environment.auth0_audience),
        (
            "PHOTARA_AUTH0_NATIVE_CLIENT_ID",
            &environment.native_client_id,
        ),
    ] {
        if get(name).is_some_and(|value| value != *expected) {
            return Err(format!("release-coordinate-drift:{name}"));
        }
    }
    let address = environment
        .bind_address(get("PORT").as_deref())
        .map_err(|_| "invalid-setting:PORT".to_owned())?;
    Ok((environment, address))
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("photara-service-startup:{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let (environment, address) = profile(|name| env::var(name).ok())?;
    let config = environment.http_config();
    let database_urls = [
        required("PHOTARA_DB_API_URL")?,
        required("PHOTARA_DB_CONTROL_URL")?,
        required("PHOTARA_DB_AUTH_READ_URL")?,
    ];
    let service = HttpService::connect(config, database_urls, Arc::new(SystemClock), cursor_key()?)
        .await
        .map_err(|_| "service-initialization-refused".to_owned())?;
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|_| "listener-bind-failed".to_owned())?;
    eprintln!("photara-service-ready:{:?}", environment.channel);
    axum::serve(
        listener,
        service
            .router()
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
    .map_err(|_| "server-failed".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_development_profile_only_and_redacted_errors() {
        let read = |name: &str| (name == "PHOTARA_RELEASE_CHANNEL").then(|| "development".into());
        assert_eq!(profile(read).unwrap().1, "127.0.0.1:8080".parse().unwrap());
        assert!(profile(|_| None).is_err());
        assert!(
            profile(|name| match name {
                "PHOTARA_RELEASE_CHANNEL" => Some("development".into()),
                "PHOTARA_DB_MIGRATION_URL" => Some("synthetic-owner-secret".into()),
                _ => None,
            })
            .is_err()
        );
        let error = profile(|name| match name {
            "PHOTARA_RELEASE_CHANNEL" => Some("development".into()),
            "PHOTARA_AUTH0_ISSUER" => Some("synthetic-untrusted-value".into()),
            _ => None,
        })
        .err()
        .unwrap();
        assert_eq!(error, "release-coordinate-drift:PHOTARA_AUTH0_ISSUER");
        assert!(!error.contains("synthetic"));
    }
}
