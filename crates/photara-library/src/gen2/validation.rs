use super::{
    Error, Result,
    types::{Extensions, PartyDetails, SocialProfile, VerificationKind},
};
use caseless::Caseless as _;
use std::collections::{BTreeMap, BTreeSet};
use unicode_normalization::UnicodeNormalization as _;

pub(super) const POLICY: &str = "photara.term-policy.v1;unicode=16.0.0;NFC;full-default-casefold;NFC;UCD-White_Space-collapse;groups=beach,beaches|studio,studios";
const _: () = assert!(
    unicode_normalization::UNICODE_VERSION.0 == 16
        && unicode_normalization::UNICODE_VERSION.1 == 0
        && unicode_normalization::UNICODE_VERSION.2 == 0
);
const _: () = assert!(
    caseless::UNICODE_VERSION.0 == 16
        && caseless::UNICODE_VERSION.1 == 0
        && caseless::UNICODE_VERSION.2 == 0
);

const fn whitespace(c: char) -> bool {
    matches!(c,'\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{0085}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}')
}
/// Frozen Unicode 16 NFC/full-default-fold/NFC/White_Space text key.
/// Semantic alias groups are claimed separately, not stemmed into this key.
/// # Errors
/// Rejects empty, control-containing or oversized normalized terms.
pub fn normalize_term(value: &str) -> Result<String> {
    bounded(value, 512)?;
    let folded: String = value.nfc().default_case_fold().nfc().collect();
    let key = folded
        .split(whitespace)
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    name(&key, 512)?;
    Ok(key)
}
pub(super) fn claims(
    display: &str,
    aliases: &BTreeSet<String>,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    if aliases.len() > 256 {
        return Err(Error::Limit);
    }
    let mut claims: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for spelling in std::iter::once(display).chain(aliases.iter().map(String::as_str)) {
        let key = normalize_term(spelling)?;
        claims
            .entry(key.clone())
            .or_default()
            .insert(spelling.to_owned());
        let group: &[&str] = match key.as_str() {
            "beach" | "beaches" => &["beach", "beaches"],
            "studio" | "studios" => &["studio", "studios"],
            _ => &[],
        };
        for term in group {
            claims
                .entry((*term).to_owned())
                .or_default()
                .insert((*term).to_owned());
        }
    }
    if claims.len() > 256 {
        return Err(Error::Limit);
    }
    Ok(claims)
}
pub(super) fn bounded(value: &str, max: usize) -> Result<()> {
    if value.len() > max {
        return Err(Error::Limit);
    }
    if value.contains('\0') {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub(super) fn name(value: &str, max: usize) -> Result<()> {
    bounded(value, max)?;
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub(super) fn identifier(value: &str) -> Result<()> {
    name(value, 256)?;
    if !value.bytes().all(|b| {
        b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-' || b == b'_'
    }) || !value.contains('.')
        || value
            .split('.')
            .any(|part| part.is_empty() || !part.as_bytes()[0].is_ascii_lowercase())
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub(super) fn set(values: &BTreeSet<String>, max: usize) -> Result<()> {
    if values.len() > 256 {
        return Err(Error::Limit);
    }
    for value in values {
        name(value, max)?;
    }
    Ok(())
}
pub(super) fn labels(values: &BTreeSet<String>) -> Result<()> {
    set(values, 256)?;
    let mut keys = BTreeSet::new();
    for value in values {
        if !keys.insert(normalize_term(value)?) {
            return Err(Error::Conflict);
        }
    }
    Ok(())
}
pub(super) fn extensions(value: &Extensions) -> Result<()> {
    for key in value.keys() {
        identifier(key)?;
    }
    let bytes =
        photara_core::canonical_json(&serde_json::to_value(value).map_err(|_| Error::Invalid)?)
            .map_err(|_| Error::Invalid)?;
    if bytes.len() > 65_536 {
        return Err(Error::Limit);
    }
    photara_store::package::parse_canonical_json(
        &bytes,
        photara_store::package::JsonLimits::default(),
    )
    .map_err(|_| Error::Invalid)?;
    Ok(())
}
pub(super) fn party(value: &PartyDetails) -> Result<()> {
    name(&value.display_name, 512)?;
    bounded(&value.description, 8192)?;
    set(&value.aliases, 512)?;
    labels(&value.labels)?;
    extensions(&value.extensions)
}
pub(super) fn relative_path(value: &str) -> Result<()> {
    photara_store::package::validate_resource_path(value).map_err(|_| Error::Invalid)
}
pub(super) fn social(value: &SocialProfile) -> Result<()> {
    identifier(&value.provider_id)?;
    if let Some(subject) = &value.subject {
        name(&subject.namespace, 256)?;
        name(&subject.subject_id, 512)?;
    }
    if let Some(handle) = &value.handle {
        name(handle, 256)?;
    }
    if value.subject.is_none() && value.handle.is_none() && value.profile_url.is_none() {
        return Err(Error::Invalid);
    }
    bounded(&value.display_name, 512)?;
    if let Some(kind) = &value.provider_account_kind {
        name(kind, 256)?;
    }
    if (value.verification == VerificationKind::Unverified) != value.verified_at.is_none() {
        return Err(Error::Invalid);
    }
    name(&value.provenance.source, 256)?;
    bounded(&value.provenance.notes, 8192)?;
    if let Some(url) = &value.profile_url {
        bounded(url, 2048)?;
        let parsed = url::Url::parse(url).map_err(|_| Error::Invalid)?;
        if parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(Error::Invalid);
        }
    }
    extensions(&value.extensions)
}
