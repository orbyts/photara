//! Checked public deployment coordinates. Secrets are never part of this descriptor.
use crate::{Result, ServiceError, http::HttpConfig, oidc::OidcConfig};
use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseChannel {
    Development,
    RemoteAcceptance,
    Production,
}

impl ReleaseChannel {
    /// # Errors
    /// Unknown channels are refused, never downgraded to development.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "development" => Ok(Self::Development),
            "remoteAcceptance" => Ok(Self::RemoteAcceptance),
            "production" => Ok(Self::Production),
            _ => Err(ServiceError::Invalid),
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductIdentity {
    pub display_name: String,
    pub short_name: String,
    pub is_codename: bool,
    pub bundle_identifier: String,
    pub keychain_service: String,
    pub callback_scheme: String,
    pub project_package_display_type: String,
    pub project_package_extension: String,
    pub api_audience_namespace: String,
    pub service_hostname: Option<String>,
    pub user_agent: String,
    pub application_support_directory: String,
    pub cache_directory: String,
    pub default_library_name: String,
    #[serde(rename = "websiteURL")]
    pub website_url: Option<String>,
    #[serde(rename = "supportURL")]
    pub support_url: Option<String>,
    #[serde(rename = "privacyURL")]
    pub privacy_url: Option<String>,
    #[serde(rename = "storeURL")]
    pub store_url: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseEnvironment {
    pub channel: ReleaseChannel,
    pub environment_id: String,
    pub api_origin: String,
    pub auth0_issuer: String,
    pub auth0_audience: String,
    pub native_client_id: String,
    #[serde(rename = "callbackURL")]
    pub callback_url: String,
    #[serde(rename = "logoutURL")]
    pub logout_url: String,
    pub schema_family: String,
    pub schema_epoch: u32,
    #[serde(rename = "minimumAPI")]
    pub minimum_api: u32,
    pub logging_policy: String,
    pub telemetry_enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Environments {
    development: ReleaseEnvironment,
    remote_acceptance: Option<ReleaseEnvironment>,
    production: Option<ReleaseEnvironment>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    identity: ProductIdentity,
    environments: Environments,
}

/// Parsed URL must be a canonical origin with no credentials or extra URL data.
fn origin(value: &str, scheme: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value).map_err(|_| ServiceError::Invalid)?;
    if url.scheme() != scheme
        || url.host_str().is_none()
        || url.path() != "/"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.as_str() != value
    {
        return Err(ServiceError::Invalid);
    }
    Ok(url)
}

fn public_https(value: &str) -> Result<()> {
    let url = origin(value, "https")?;
    let host = url.host_str().ok_or(ServiceError::Invalid)?;
    // Remote origins are checked DNS names, never literal/private/loopback addresses.
    if !host.contains('.')
        || host.ends_with('.')
        || host.ends_with(".localhost")
        || host.rsplit('.').next() == Some("local")
        || host.trim_matches(['[', ']']).parse::<IpAddr>().is_ok()
    {
        return Err(ServiceError::Invalid);
    }
    Ok(())
}

impl Descriptor {
    fn select(self, channel: ReleaseChannel) -> Result<(ProductIdentity, ReleaseEnvironment)> {
        let development = &self.environments.development;
        let selected = match channel {
            ReleaseChannel::Development => development.clone(),
            ReleaseChannel::RemoteAcceptance => self
                .environments
                .remote_acceptance
                .clone()
                .ok_or(ServiceError::Unsupported)?,
            ReleaseChannel::Production => self
                .environments
                .production
                .clone()
                .ok_or(ServiceError::Unsupported)?,
        };
        if selected.channel != channel
            || selected.environment_id.is_empty()
            || selected.environment_id.len() > 128
            || selected.schema_family != "photara.service.g2"
            || selected.schema_epoch != 1
            || selected.minimum_api != 3
            || selected.logging_policy != "redacted-status-only"
            || selected.telemetry_enabled
            || selected.native_client_id.is_empty()
            || selected.auth0_audience.is_empty()
        {
            return Err(ServiceError::Invalid);
        }
        public_https(&selected.auth0_issuer)?;
        let issuer =
            reqwest::Url::parse(&selected.auth0_issuer).map_err(|_| ServiceError::Invalid)?;
        let callback = format!(
            "{}://{}/macos/{}/callback",
            self.identity.callback_scheme,
            issuer.host_str().ok_or(ServiceError::Invalid)?,
            self.identity.bundle_identifier
        );
        if selected.callback_url != callback || selected.logout_url != callback {
            return Err(ServiceError::Invalid);
        }
        match channel {
            ReleaseChannel::Development => {
                let url = origin(&selected.api_origin, "http")?;
                if url.host_str() != Some("127.0.0.1") || url.port().is_none_or(|port| port == 0) {
                    return Err(ServiceError::Invalid);
                }
            }
            ReleaseChannel::RemoteAcceptance | ReleaseChannel::Production => {
                public_https(&selected.api_origin)?;
                if selected.environment_id == development.environment_id {
                    return Err(ServiceError::Invalid);
                }
                if channel == ReleaseChannel::Production
                    && (self.identity.is_codename
                        || selected.auth0_issuer == development.auth0_issuer
                        || selected.auth0_audience == development.auth0_audience
                        || selected.native_client_id == development.native_client_id
                        || selected.callback_url == development.callback_url
                        || selected.logout_url == development.logout_url
                        || selected.callback_url.split(':').next()
                            == development.callback_url.split(':').next())
                {
                    return Err(ServiceError::Invalid);
                }
            }
        }
        Ok((self.identity, selected))
    }
}

/// # Errors
/// Unprovisioned channels and inconsistent checked coordinates fail closed.
pub fn checked(channel: ReleaseChannel) -> Result<(ProductIdentity, ReleaseEnvironment)> {
    let descriptor: Descriptor =
        serde_json::from_str(include_str!("../../../config/product-identity.json"))
            .map_err(|_| ServiceError::Invalid)?;
    descriptor.select(channel)
}

impl ReleaseEnvironment {
    #[must_use]
    pub fn http_config(&self) -> HttpConfig {
        HttpConfig {
            release_channel: self.channel,
            environment_id: self.environment_id.clone(),
            service_origin: self.api_origin.clone(),
            oidc: OidcConfig {
                issuer: self.auth0_issuer.clone(),
                audience: self.auth0_audience.clone(),
                native_client_id: self.native_client_id.clone(),
                // Auth0 adds this issuer's /userinfo audience when the native
                // authorization request includes openid and the custom API.
                allow_userinfo_audience: true,
            },
        }
    }

    /// # Errors
    /// A development listener cannot be redirected to another port or interface.
    pub fn bind_address(&self, port: Option<&str>) -> Result<SocketAddr> {
        let development_port = reqwest::Url::parse(&self.api_origin)
            .map_err(|_| ServiceError::Invalid)?
            .port_or_known_default()
            .ok_or(ServiceError::Invalid)?;
        let supplied: u16 = port
            .unwrap_or("8080")
            .parse()
            .map_err(|_| ServiceError::Invalid)?;
        if supplied == 0 {
            return Err(ServiceError::Invalid);
        }
        match self.channel {
            ReleaseChannel::Development => {
                if port.is_some() && supplied != development_port {
                    return Err(ServiceError::Invalid);
                }
                Ok(SocketAddr::from(([127, 0, 0, 1], development_port)))
            }
            _ => Ok(SocketAddr::from(([0, 0, 0, 0], supplied))),
        }
    }
}

/// # Errors
/// Callers cannot substitute ad hoc origins or token trust coordinates.
pub fn validate_http(config: &HttpConfig) -> Result<()> {
    let (_, expected) = checked(config.release_channel)?;
    if config.environment_id != expected.environment_id
        || config.service_origin != expected.api_origin
        || config.oidc.issuer != expected.auth0_issuer
        || config.oidc.audience != expected.auth0_audience
        || config.oidc.native_client_id != expected.native_client_id
        || config.oidc.allow_userinfo_audience
            != expected.http_config().oidc.allow_userinfo_audience
    {
        return Err(ServiceError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SyntheticClock;
    impl crate::auth::Clock for SyntheticClock {
        fn now_ms(&self) -> i64 {
            1_800_000_000_000
        }
    }

    #[tokio::test]
    async fn loopback_retains_certificate_verified_database_admission() {
        let (_, environment) = checked(ReleaseChannel::Development).unwrap();
        // Rejected before JWKS warm-up or any connection. These are inert values.
        for query in [
            "",
            "?sslmode=require",
            "?sslmode=disable",
            "?sslmode=verify-full&sslmode=require",
        ] {
            let urls = std::array::from_fn(|_| {
                format!("postgresql://synthetic@database.invalid/test{query}")
            });
            let result = crate::http::HttpService::connect(
                environment.http_config(),
                urls,
                std::sync::Arc::new(SyntheticClock),
                [7; 32],
            )
            .await;
            assert!(matches!(result, Err(ServiceError::Invalid)));
        }
    }

    #[test]
    fn checked_development_binds_only_loopback() {
        let (identity, environment) = checked(ReleaseChannel::Development).unwrap();
        assert_eq!(identity.display_name, "Photara");
        assert_eq!(
            environment.bind_address(None).unwrap(),
            "127.0.0.1:8080".parse().unwrap()
        );
        assert!(environment.bind_address(Some("0")).is_err());
        assert!(environment.bind_address(Some("8081")).is_err());
        validate_http(&environment.http_config()).unwrap();
        assert!(environment.http_config().oidc.allow_userinfo_audience);
        let mut stale = environment.http_config();
        stale.oidc.allow_userinfo_audience = false;
        assert!(validate_http(&stale).is_err());
        let mut drift = environment.http_config();
        drift.service_origin = "http://0.0.0.0:8080/".into();
        assert!(validate_http(&drift).is_err());
        let mut drift = environment.http_config();
        drift.oidc.native_client_id = "other-client".into();
        assert!(validate_http(&drift).is_err());
    }

    #[test]
    fn unprovisioned_release_channels_fail_closed() {
        assert!(checked(ReleaseChannel::Production).is_err());
        assert!(checked(ReleaseChannel::RemoteAcceptance).is_err());
        assert!(ReleaseChannel::parse("prodution").is_err());
    }

    #[test]
    fn remote_and_production_refuse_loopback_and_trust_drift() {
        let raw = include_str!("../../../config/product-identity.json");
        for channel in [ReleaseChannel::RemoteAcceptance, ReleaseChannel::Production] {
            for bad in [
                "http://127.0.0.1:8080/",
                "https://127.0.0.1/",
                "https://[::1]/",
                "https://localhost/",
                "https://localhost./",
                "https://api.local./",
                "https://api.local/",
                "https://api.example.com/path",
                "https://api.example.com/?x=1",
            ] {
                let mut descriptor: Descriptor = serde_json::from_str(raw).unwrap();
                let mut env = descriptor.environments.development.clone();
                env.channel = channel;
                env.environment_id = "remote".into();
                env.api_origin = bad.into();
                descriptor.environments.remote_acceptance = Some(env.clone());
                descriptor.environments.production = Some(env);
                assert!(descriptor.select(channel).is_err(), "{bad}");
            }
        }
        let mut descriptor: Descriptor = serde_json::from_str(raw).unwrap();
        let mut env = descriptor.environments.development.clone();
        env.channel = ReleaseChannel::Production;
        env.environment_id = "production".into();
        env.api_origin = "https://api.example.com/".into();
        descriptor.identity.is_codename = false;
        descriptor.environments.production = Some(env);
        assert!(descriptor.select(ReleaseChannel::Production).is_err());
    }
}
