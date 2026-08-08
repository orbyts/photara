use keyring::{Entry, Error as KeyringError};

use crate::{PhotaraError, Result};

const SERVICE: &str = "io.github.orbyts.photara.credentials";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretId {
    provider: String,
    account: String,
    kind: String,
}

impl SecretId {
    pub fn new(provider: &str, account: &str, kind: &str) -> Result<Self> {
        for (label, value) in [
            ("provider", provider),
            ("account", account),
            ("credential kind", kind),
        ] {
            if value.trim().is_empty() || value.contains(':') {
                return Err(PhotaraError::Configuration(format!(
                    "credential {label} must be non-empty and cannot contain ':'"
                )));
            }
        }
        Ok(Self {
            provider: provider.into(),
            account: account.into(),
            kind: kind.into(),
        })
    }

    fn username(&self) -> String {
        format!("{}:{}:{}", self.provider, self.account, self.kind)
    }
}

pub trait CredentialStore {
    fn save(&self, id: &SecretId, secret: &[u8]) -> Result<()>;
    fn load(&self, id: &SecretId) -> Result<Option<Vec<u8>>>;
    fn delete(&self, id: &SecretId) -> Result<bool>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemCredentialStore;

impl SystemCredentialStore {
    fn entry(id: &SecretId) -> Result<Entry> {
        Entry::new(SERVICE, &id.username()).map_err(credential_error)
    }
}

impl CredentialStore for SystemCredentialStore {
    fn save(&self, id: &SecretId, secret: &[u8]) -> Result<()> {
        Self::entry(id)?
            .set_secret(secret)
            .map_err(credential_error)
    }

    fn load(&self, id: &SecretId) -> Result<Option<Vec<u8>>> {
        match Self::entry(id)?.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(credential_error(error)),
        }
    }

    fn delete(&self, id: &SecretId) -> Result<bool> {
        match Self::entry(id)?.delete_credential() {
            Ok(()) => Ok(true),
            Err(KeyringError::NoEntry) => Ok(false),
            Err(error) => Err(credential_error(error)),
        }
    }
}

fn credential_error(error: KeyringError) -> PhotaraError {
    PhotaraError::Credential(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_identity_is_provider_and_account_scoped() {
        let id = SecretId::new("adobe-lightroom", "personal", "refresh-token").unwrap();
        assert_eq!(id.username(), "adobe-lightroom:personal:refresh-token");
        assert!(SecretId::new("adobe:lightroom", "personal", "refresh-token").is_err());
    }
}
