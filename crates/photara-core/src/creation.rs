//! UI1 portable initial-authoring contract. Host destinations remain outside Core.
use crate::contracts::{CommitId, GraphId, LibraryId, OperationId, ProjectId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialProject {
    pub operation_id: OperationId,
    pub library_id: LibraryId,
    pub project_id: ProjectId,
    pub graph_id: GraphId,
    pub commit_id: CommitId,
    pub title: String,
    pub created_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("Enter a project name.")]
    Empty,
    #[error("The project name contains characters that cannot be used in a package name.")]
    Unsafe,
    #[error("The project name is too long for a package filename.")]
    TooLong,
}

/// Trim without lossy sanitization. Identical titles may exist at different destinations.
/// # Errors
/// Rejects empty, unsafe/reserved or overlong portable filenames.
pub fn project_name(input: &str) -> Result<String, NameError> {
    let title = input.trim();
    if title.is_empty() {
        return Err(NameError::Empty);
    }
    if title.len() > 255 {
        return Err(NameError::TooLong);
    }
    let stem = title
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        });
    if title.starts_with('.')
        || title.ends_with('.')
        || device
        || title.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
    {
        return Err(NameError::Unsafe);
    }
    Ok(title.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_names_preserve_unicode_and_reject_collapsing_sanitization() {
        assert_eq!(project_name(" \n Café 夏 \t").unwrap(), "Café 夏");
        for name in [
            "", " \t", ".", "..", ".hidden", "a/b", "a\\b", "a:b", "a\0b", "a\nb", "a?b", "a.",
            "con", "NUL.txt", "COM1", "lpt9.ext",
        ] {
            assert!(project_name(name).is_err(), "{name:?}");
        }
        assert!(project_name(&"a".repeat(255)).is_ok());
        assert_eq!(project_name(&"a".repeat(256)), Err(NameError::TooLong));
        assert!(project_name("CONifer").is_ok());
    }
}

/// Validated outer filename policy, never part of portable package bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PackageExtension(String);

impl TryFrom<String> for PackageExtension {
    type Error = NameError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.len() > 32
            || !value.as_bytes()[0].is_ascii_lowercase()
            || !value
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        {
            return Err(NameError::Unsafe);
        }
        Ok(Self(value))
    }
}
impl From<PackageExtension> for String {
    fn from(value: PackageExtension) -> Self {
        value.0
    }
}
impl PackageExtension {
    /// Compatibility default for already persisted UI1 requests only.
    /// New requests must use generated public configuration.
    #[must_use]
    pub fn legacy_creation_alias() -> Self {
        Self("photara".into())
    }

    /// Preserve the old local request encoding under the unchanged development config.
    #[must_use]
    pub fn is_legacy_creation_alias(&self) -> bool {
        self == &Self::legacy_creation_alias()
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Build an outer filename using the actual suffix byte budget.
    /// # Errors
    /// Rejects unsafe names and filenames over 255 UTF-8 bytes.
    pub fn filename(&self, title: &str) -> Result<String, NameError> {
        let title = project_name(title)?;
        let filename = format!("{title}.{}", self.0);
        if filename.len() > 255 {
            return Err(NameError::TooLong);
        }
        Ok(filename)
    }
}

#[cfg(test)]
mod naming_tests {
    use super::*;
    #[test]
    fn extension_budget_and_unsafe_configuration_are_checked() {
        let extension = PackageExtension::try_from("jprtest".to_owned()).unwrap();
        assert!(extension.filename(&"a".repeat(247)).is_ok());
        assert_eq!(
            extension.filename(&"a".repeat(248)),
            Err(NameError::TooLong)
        );
        let longer = PackageExtension::try_from("longersuffix".to_owned()).unwrap();
        assert_eq!(longer.filename(&"a".repeat(247)), Err(NameError::TooLong));
        for bad in [
            "", ".photara", "../bad", "a/b", "a\\b", "UPPER", "foo.bar", "a b",
        ] {
            assert!(PackageExtension::try_from(bad.to_owned()).is_err());
            assert!(serde_json::from_value::<PackageExtension>(serde_json::json!(bad)).is_err());
        }
    }
}
