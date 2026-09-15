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
    if title.len() + ".photara".len() > 255 {
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
        assert!(project_name(&"a".repeat(247)).is_ok());
        assert_eq!(project_name(&"a".repeat(248)), Err(NameError::TooLong));
        assert!(project_name("CONifer").is_ok());
    }
}
