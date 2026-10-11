//! Scoped device-local history reads. Hashes authenticate requested bytes; the
//! registered adapter separately supplies namespace custody and authority.
use super::super::wire;
use super::{CheckpointIdentity, LocalArchive, LocalBase, check, common};
use crate::package::v1_3::{ReaderLimits, selected, tree};
use crate::package::{PackageError, PackageUuid, parse_canonical_json};
use serde_json::Value;
use std::collections::BTreeSet;

/// Bounded reads from the independently registered immutable local namespace.
/// Implementations retain safe handle/name pins and reject changed identities.
/// This does not grant a writer lease or make a parsed record authoritative.
pub trait CheckpointRecords: Send + Sync {
    /// Return exactly one retained canonical record, bounded before allocation.
    /// # Errors
    /// Refuses missing names, changed pins, short reads or the requested bound.
    fn record(&self, reference: &Value, max_bytes: usize) -> Result<Vec<u8>, PackageError>;
    /// Read exactly one retained frame range without scanning the whole segment.
    /// # Errors
    /// Refuses unregistered streams, changed pins, short or out-of-extent reads.
    fn frame(
        &self,
        stream: PackageUuid,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, PackageError>;
}
struct Read<'a, R: ?Sized> {
    source: &'a R,
    identity: &'a CheckpointIdentity,
    limits: &'a ReaderLimits,
    seen: BTreeSet<wire::Key>,
}
impl<R: CheckpointRecords + ?Sized> Read<'_, R> {
    fn collect(&mut self, reference: &Value, depth: usize) -> Result<Vec<Value>, PackageError> {
        if depth > self.limits.tree.max_depth {
            return Err(PackageError::Limit);
        }
        let page = self.record(reference)?;
        let leaf = page["schema"]["id"] == "photara.session.checkpoint-operation-lookup-leaf";
        common(
            &page,
            if leaf {
                "photara.session.checkpoint-operation-lookup-leaf"
            } else {
                "photara.session.checkpoint-operation-lookup-branch"
            },
            if leaf {
                &["count", "entries"]
            } else {
                &["count", "children"]
            },
            self.identity,
        )?;
        let mut entries = Vec::new();
        if leaf {
            let rows = page["entries"].as_array().ok_or(PackageError::Record)?;
            if rows.len() > self.limits.tree.max_leaf_entries {
                return Err(PackageError::Limit);
            }
            entries.extend(rows.iter().cloned());
        } else {
            let rows = page["children"].as_array().ok_or(PackageError::Record)?;
            check(rows.len() >= 2)?;
            if rows.len() > self.limits.tree.max_branch_children {
                return Err(PackageError::Limit);
            }
            for row in rows {
                tree::fields(row, &["first", "last", "count", "child"])?;
                let children = self.collect(&row["child"], depth + 1)?;
                check(
                    !children.is_empty()
                        && children.first().ok_or(PackageError::Integrity)?["operation_id"]
                            == row["first"]
                        && children.last().ok_or(PackageError::Integrity)?["operation_id"]
                            == row["last"]
                        && children.len() as u64 == tree::number(&row["count"])?,
                )?;
                if entries
                    .len()
                    .checked_add(children.len())
                    .ok_or(PackageError::Limit)?
                    > self.limits.tree.max_entries
                {
                    return Err(PackageError::Limit);
                }
                entries.extend(children);
            }
        }
        if entries.len() > self.limits.tree.max_entries {
            return Err(PackageError::Limit);
        }
        check(entries.len() as u64 == tree::number(&page["count"])?)?;
        let mut prior = None;
        for entry in &entries {
            let operation = Self::entry(entry)?;
            check(prior.is_none_or(|old| old < operation))?;
            prior = Some(operation);
        }
        Ok(entries)
    }
    fn record(&mut self, reference: &Value) -> Result<Value, PackageError> {
        selected::json_ref(reference)?;
        let key = wire::key(reference).map_err(|_| PackageError::Record)?;
        check(self.seen.insert(key))?;
        if self.seen.len() > self.limits.tree.max_pages {
            return Err(PackageError::Limit);
        }
        let length = usize::try_from(tree::number(&reference["byte_length"])?)
            .map_err(|_| PackageError::Limit)?;
        if length > self.limits.json.max_bytes {
            return Err(PackageError::Limit);
        }
        let bytes = self.source.record(reference, length)?;
        check(bytes.len() == length && wire::hash(&bytes) == reference["sha256"])?;
        parse_canonical_json(&bytes, self.limits.json)
    }
    fn entry(entry: &Value) -> Result<PackageUuid, PackageError> {
        tree::fields(
            entry,
            &["operation_id", "request_sha256", "receipt", "local"],
        )?;
        let operation = selected::nonnil(&entry["operation_id"])?;
        selected::sha(&entry["request_sha256"])?;
        selected::json_ref(&entry["receipt"])?;
        let local = &entry["local"];
        if !local.is_null() {
            tree::fields(local, &["mutation", "owner_sha256", "action_sha256"])?;
            selected::sha(&local["owner_sha256"])?;
            selected::sha(&local["action_sha256"])?;
            let archived = &local["mutation"];
            tree::fields(
                archived,
                &["stream_id", "offset", "byte_length", "record", "checksum"],
            )?;
            selected::nonnil(&archived["stream_id"])?;
            selected::json_ref(&archived["record"])?;
            selected::sha(&archived["checksum"])?;
            tree::number(&archived["offset"])?;
            check(tree::number(&archived["byte_length"])? >= 36)?;
        }
        Ok(operation)
    }
    fn lookup(
        &mut self,
        mut reference: Value,
        operation: PackageUuid,
    ) -> Result<Option<Value>, PackageError> {
        let mut expected: Option<(PackageUuid, PackageUuid, u64)> = None;
        for depth in 0..=self.limits.tree.max_depth {
            let page = self.record(&reference)?;
            let count = tree::number(&page["count"])?;
            if count > self.limits.tree.max_entries as u64 {
                return Err(PackageError::Limit);
            }
            let leaf = page["schema"]["id"] == "photara.session.checkpoint-operation-lookup-leaf";
            common(
                &page,
                if leaf {
                    "photara.session.checkpoint-operation-lookup-leaf"
                } else {
                    "photara.session.checkpoint-operation-lookup-branch"
                },
                if leaf {
                    &["count", "entries"]
                } else {
                    &["count", "children"]
                },
                self.identity,
            )?;
            let (first, last, actual, found) = if leaf {
                let entries = page["entries"].as_array().ok_or(PackageError::Record)?;
                if entries.len() > self.limits.tree.max_leaf_entries {
                    return Err(PackageError::Limit);
                }
                let mut previous = None;
                let mut first = None;
                let mut found = None;
                for entry in entries {
                    let key = Self::entry(entry)?;
                    check(previous.is_none_or(|old| old < key))?;
                    first.get_or_insert(key);
                    previous = Some(key);
                    if key == operation {
                        found = Some(entry.clone());
                    }
                }
                (first, previous, entries.len() as u64, found)
            } else {
                let rows = page["children"].as_array().ok_or(PackageError::Record)?;
                check(rows.len() >= 2)?;
                if rows.len() > self.limits.tree.max_branch_children {
                    return Err(PackageError::Limit);
                }
                let mut first = None;
                let mut last = None;
                let mut total = 0u64;
                let mut found = None;
                for row in rows {
                    tree::fields(row, &["first", "last", "count", "child"])?;
                    let low = selected::nonnil(&row["first"])?;
                    let high = selected::nonnil(&row["last"])?;
                    let size = tree::number(&row["count"])?;
                    selected::json_ref(&row["child"])?;
                    check(low <= high && size > 0 && last.is_none_or(|old| old < low))?;
                    total = total.checked_add(size).ok_or(PackageError::Limit)?;
                    first.get_or_insert(low);
                    last = Some(high);
                    if low <= operation && operation <= high {
                        found = Some(row.clone());
                    }
                }
                (first, last, total, found)
            };
            check(count == actual)?;
            if let Some((low, high, size)) = expected {
                check(first == Some(low) && last == Some(high) && actual == size)?;
            }
            if leaf || found.is_none() {
                return Ok(found);
            }
            if depth == self.limits.tree.max_depth {
                return Err(PackageError::Limit);
            }
            let row = found.ok_or(PackageError::Integrity)?;
            expected = Some((
                selected::nonnil(&row["first"])?,
                selected::nonnil(&row["last"])?,
                tree::number(&row["count"])?,
            ));
            reference = row["child"].clone();
        }
        Err(PackageError::Limit)
    }
    fn segment(
        &mut self,
        mut reference: Value,
        stream: PackageUuid,
    ) -> Result<Value, PackageError> {
        for _ in 0..=self.limits.tree.max_depth {
            let value = self.record(&reference)?;
            let archive =
                LocalArchive::parse(&wire::encode(&value), self.identity, self.limits.json)?;
            for segment in archive.value()["segments"]
                .as_array()
                .ok_or(PackageError::Record)?
            {
                if segment["stream_id"] == stream.to_string() {
                    return Ok(segment.clone());
                }
            }
            reference = archive.value()["previous"].clone();
            if reference.is_null() {
                return Err(PackageError::Integrity);
            }
        }
        Err(PackageError::Limit)
    }
}
impl LocalBase {
    pub(crate) fn entries(
        &self,
        source: &(impl CheckpointRecords + ?Sized),
        identity: &CheckpointIdentity,
        limits: &ReaderLimits,
    ) -> Result<Vec<Value>, PackageError> {
        LocalBase::parse(&wire::encode(self.value()), identity, limits.json)?;
        Read {
            source,
            identity,
            limits,
            seen: BTreeSet::new(),
        }
        .collect(&self.value()["lookup"], 0)
    }
    /// Resolve one authenticated local dedupe path, without reading unrelated
    /// archive payloads. A null local member proves no local ownership.
    /// # Errors
    /// Refuses changed records, identities, ranges, ordering, counts or bounds.
    pub fn lookup_operation(
        &self,
        source: &(impl CheckpointRecords + ?Sized),
        identity: &CheckpointIdentity,
        operation: PackageUuid,
        limits: &ReaderLimits,
    ) -> Result<Option<Value>, PackageError> {
        LocalBase::parse(&wire::encode(self.value()), identity, limits.json)?;
        Read {
            source,
            identity,
            limits,
            seen: BTreeSet::new(),
        }
        .lookup(self.value()["lookup"].clone(), operation)
    }
    /// Resolve the exact original Mutation through its authenticated local path.
    /// The caller additionally cross-checks the portable package receipt before
    /// acknowledging retry; this method cannot manufacture acceptance or Saved.
    /// # Errors
    /// Refuses wrong request, owner/action, archive range, frame or receipt link.
    pub fn lookup_mutation(
        &self,
        source: &(impl CheckpointRecords + ?Sized),
        identity: &CheckpointIdentity,
        operation: PackageUuid,
        limits: &ReaderLimits,
    ) -> Result<Option<Value>, PackageError> {
        LocalBase::parse(&wire::encode(self.value()), identity, limits.json)?;
        let mut read = Read {
            source,
            identity,
            limits,
            seen: BTreeSet::new(),
        };
        let Some(entry) = read.lookup(self.value()["lookup"].clone(), operation)? else {
            return Ok(None);
        };
        if entry["local"].is_null() {
            return Ok(None);
        }
        let local = &entry["local"];
        let archived = &local["mutation"];
        let stream = selected::nonnil(&archived["stream_id"])?;
        let segment = read.segment(self.value()["archive"].clone(), stream)?;
        let offset = tree::number(&archived["offset"])?;
        let length = tree::number(&archived["byte_length"])?;
        check(
            offset >= 44
                && offset
                    .checked_add(length)
                    .is_some_and(|end| end <= tree::number(&segment["byte_length"]).unwrap_or(0)),
        )?;
        let length = usize::try_from(length).map_err(|_| PackageError::Limit)?;
        if length
            > limits
                .json
                .max_bytes
                .checked_add(36)
                .ok_or(PackageError::Limit)?
        {
            return Err(PackageError::Limit);
        }
        let bytes = source.frame(stream, offset, length)?;
        check(bytes.len() == length && length >= 36)?;
        let payload_len =
            u32::from_le_bytes(bytes[..4].try_into().map_err(|_| PackageError::Record)?) as usize;
        check(payload_len.checked_add(36) == Some(length))?;
        let payload = &bytes[4..length - 32];
        check(
            wire::hash(payload) == archived["record"]["sha256"]
                && payload.len() as u64 == tree::number(&archived["record"]["byte_length"])?
                && hex(&bytes[length - 32..]) == archived["checksum"],
        )?;
        let record = parse_canonical_json(payload, limits.json)?;
        super::super::journal::validate_archived_mutation(&record, identity)?;
        let sequence = tree::number(&record["sequence"])?;
        check(
            sequence >= tree::number(&segment["first_sequence"])?
                && sequence <= tree::number(&segment["last_sequence"])?
                && segment["journal_id"] == self.value()["journal_id"],
        )?;
        let body = &record["body"];
        check(
            body["operation_receipt"]["operation_id"] == operation.to_string()
                && body["operation_receipt"]["request_sha256"] == entry["request_sha256"]
                && wire::reference(&body["operation_receipt"]) == entry["receipt"]
                && wire::hash(&wire::encode(&body["owner"])) == local["owner_sha256"]
                && wire::hash(&wire::encode(&body["action"])) == local["action_sha256"],
        )?;
        Ok(Some(record))
    }
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[(byte >> 4) as usize]),
                char::from(DIGITS[(byte & 15) as usize]),
            ]
        })
        .collect()
}

/// Retains only immutable roots and a registered reader, never archive contents.
pub(crate) struct History {
    pub(crate) base: LocalBase,
    pub(crate) source: std::sync::Arc<dyn CheckpointRecords>,
    pub(crate) identity: CheckpointIdentity,
    pub(crate) limits: ReaderLimits,
}
impl History {
    pub(crate) fn contains(&self, operation: &str) -> Result<bool, PackageError> {
        let id = PackageUuid::parse(operation)?;
        Ok(self
            .base
            .lookup_operation(self.source.as_ref(), &self.identity, id, &self.limits)?
            .is_some())
    }
    pub(crate) fn mutation(&self, operation: &str) -> Result<Option<Value>, PackageError> {
        let id = PackageUuid::parse(operation)?;
        self.base
            .lookup_mutation(self.source.as_ref(), &self.identity, id, &self.limits)
    }
}
