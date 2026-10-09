//! Exact frozen PS2 framing and direct physical coordinates.
use std::{collections::BTreeMap, sync::Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::{
    DecimalU64, JsonLimits, PackageError, PackageUuid, Sha256Hex, digest, parse_canonical_json,
};

const HEADER: usize = 16;
const MAGIC: &[u8; 8] = b"PS2PKD01";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Arena {
    Data,
    Metadata,
}

/// Padding owns bytes but is never a referenceable object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FrameKind {
    Padding = 0,
    Semantic = 1,
    Ownership = 2,
    Physical = 3,
}
impl FrameKind {
    fn decode(tag: u8, arena: Arena) -> Result<Self, PackageError> {
        match (tag, arena) {
            (0, _) => Ok(Self::Padding),
            (1, Arena::Data) => Ok(Self::Semantic),
            (2, Arena::Data) => Ok(Self::Ownership),
            (3, Arena::Metadata) => Ok(Self::Physical),
            _ => Err(PackageError::Record),
        }
    }
}

/// Direct coordinate; the digest authenticates the entire frame including header.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalRef {
    pub allocation_id: PackageUuid,
    pub arena: Arena,
    pub offset: DecimalU64,
    pub byte_length: DecimalU64,
    pub record_sha256: Sha256Hex,
}

/// Explicit process budgets, deliberately without fixture-derived defaults.
#[derive(Clone, Copy, Debug)]
pub struct FrameLimits {
    pub json: JsonLimits,
    pub max_allocation_bytes: usize,
    pub max_frames: usize,
}

/// A checked frame-boundary index over borrowed immutable bytes.
///
/// Construction validates framing, arenas, padding and bounds. It deliberately
/// does not parse/hash unaccessed JSON frames. `resolve` authenticates and parses
/// an accessed record; successful decoding is memoized only for this immutable
/// allocation and its fixed parser limits. The memo contains at most the indexed
/// non-padding frames (bounded by `max_frames` and `max_allocation_bytes`).
/// Full retained-closure auditing is a separate operation.
pub struct PackedAllocation<'a> {
    allocation_id: PackageUuid,
    arena: Arena,
    bytes: &'a [u8],
    frames: BTreeMap<u64, (u64, FrameKind)>,
    limits: FrameLimits,
    decoded: Mutex<BTreeMap<u64, (Sha256Hex, Value)>>,
}
impl<'a> PackedAllocation<'a> {
    /// # Errors
    /// Refuses nil identity, malformed/truncated frames, nonzero padding,
    /// unsupported tag/arena pairs and caller budget exhaustion.
    pub fn open(
        allocation_id: PackageUuid,
        arena: Arena,
        bytes: &'a [u8],
        limits: FrameLimits,
    ) -> Result<Self, PackageError> {
        if allocation_id.as_uuid().is_nil() {
            return Err(PackageError::Record);
        }
        if bytes.len() > limits.max_allocation_bytes {
            return Err(PackageError::Limit);
        }
        let mut frames = BTreeMap::new();
        let mut at = 0usize;
        while at < bytes.len() {
            if frames.len() >= limits.max_frames {
                return Err(PackageError::Limit);
            }
            let body_start = at.checked_add(HEADER).ok_or(PackageError::Limit)?;
            let header = bytes.get(at..body_start).ok_or(PackageError::Integrity)?;
            if &header[..8] != MAGIC || header[9..12] != [0; 3] {
                return Err(PackageError::Record);
            }
            let kind = FrameKind::decode(header[8], arena)?;
            let length = u32::from_le_bytes(
                header[12..16]
                    .try_into()
                    .map_err(|_| PackageError::Record)?,
            ) as usize;
            let end = body_start.checked_add(length).ok_or(PackageError::Limit)?;
            let body = bytes.get(body_start..end).ok_or(PackageError::Integrity)?;
            if kind == FrameKind::Padding {
                if body.iter().any(|b| *b != 0) {
                    return Err(PackageError::Integrity);
                }
            } else if length == 0 {
                return Err(PackageError::Record);
            } else if length > limits.json.max_bytes {
                return Err(PackageError::Limit);
            }
            frames.insert(at as u64, ((end - at) as u64, kind));
            at = end;
        }
        Ok(Self {
            allocation_id,
            arena,
            bytes,
            frames,
            limits,
            decoded: Mutex::new(BTreeMap::new()),
        })
    }

    /// # Errors
    /// Requires the exact allocation, arena, frame boundary, length, membership
    /// tag and digest. Padding, interior slices and noncanonical JSON refuse.
    pub fn resolve(&self, reference: &PhysicalRef, kind: FrameKind) -> Result<Value, PackageError> {
        if reference.allocation_id != self.allocation_id
            || reference.arena != self.arena
            || kind == FrameKind::Padding
            || self.frames.get(&reference.offset.get())
                != Some(&(reference.byte_length.get(), kind))
        {
            return Err(PackageError::Integrity);
        }
        // The frame index, immutable byte slice and parser limits belong to this
        // instance. Cache only successful authentication; callers still supply
        // the exact identity, arena, boundary, length, tag and digest on every use.
        let mut decoded = self.decoded.lock().map_err(|_| PackageError::Integrity)?;
        if let Some((hash, value)) = decoded.get(&reference.offset.get()) {
            if hash != &reference.record_sha256 {
                return Err(PackageError::Integrity);
            }
            return Ok(value.clone());
        }
        let value = self.decode(reference)?;
        decoded.insert(
            reference.offset.get(),
            (reference.record_sha256.clone(), value.clone()),
        );
        Ok(value)
    }

    fn decode(&self, reference: &PhysicalRef) -> Result<Value, PackageError> {
        let start = usize::try_from(reference.offset.get()).map_err(|_| PackageError::Limit)?;
        let length =
            usize::try_from(reference.byte_length.get()).map_err(|_| PackageError::Limit)?;
        let end = start.checked_add(length).ok_or(PackageError::Limit)?;
        let frame = self.bytes.get(start..end).ok_or(PackageError::Integrity)?;
        if digest(frame) != reference.record_sha256 {
            return Err(PackageError::Integrity);
        }
        parse_canonical_json(&frame[HEADER..], self.limits.json)
    }
}

/// Builds the frozen frame bytes in memory. No allocation, journal or package
/// publication is performed, and this does not grant writer admission.
///
/// # Errors
/// Refuses wrong arena, malformed/noncanonical JSON, nonzero padding, overflow
/// and caller budget exhaustion. The supplied bytes are retained exactly.
pub fn encode_frame(
    arena: Arena,
    kind: FrameKind,
    body: &[u8],
    limits: FrameLimits,
) -> Result<Vec<u8>, PackageError> {
    FrameKind::decode(kind as u8, arena)?;
    let length = u32::try_from(body.len()).map_err(|_| PackageError::Limit)?;
    let total = body.len().checked_add(HEADER).ok_or(PackageError::Limit)?;
    if total > limits.max_allocation_bytes || limits.max_frames == 0 {
        return Err(PackageError::Limit);
    }
    if kind == FrameKind::Padding {
        if body.iter().any(|b| *b != 0) {
            return Err(PackageError::Integrity);
        }
    } else {
        parse_canonical_json(body, limits.json)?;
    }
    let mut frame = Vec::new();
    frame
        .try_reserve_exact(total)
        .map_err(|_| PackageError::Limit)?;
    frame.extend_from_slice(MAGIC);
    frame.extend_from_slice(&[kind as u8, 0, 0, 0]);
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(body);
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{hint::black_box, time::Instant};

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "One existing-corpus workload checks memo equivalence, isolation, refusal and timing together."
    )]
    fn immutable_frame_memo_matches_uncached_frozen_records() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../../docs/architecture/proposals/ps2/selected-blob/linked.json"
        ))
        .unwrap();
        let limits = FrameLimits {
            json: JsonLimits {
                max_bytes: 1 << 20,
                max_depth: 64,
                max_members: 4096,
                max_array_elements: 4096,
            },
            max_allocation_bytes: 16 << 20,
            max_frames: 65_536,
        };
        let mut uncached_time = std::time::Duration::ZERO;
        let mut cached_time = std::time::Duration::ZERO;
        let mut resolved = 0;
        for (id, allocation) in fixture["allocations"].as_object().unwrap() {
            if allocation["layout"] == "whole-blob" {
                continue;
            }
            let hex = allocation["hex"].as_str().unwrap();
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>();
            let arena: Arena = serde_json::from_value(allocation["arena"].clone()).unwrap();
            let checked =
                PackedAllocation::open(PackageUuid::parse(id).unwrap(), arena, &bytes, limits)
                    .unwrap();
            let references = checked
                .frames
                .iter()
                .filter(|(_, (_, kind))| *kind != FrameKind::Padding)
                .map(|(offset, (length, kind))| {
                    let frame = &bytes[usize::try_from(*offset).unwrap()
                        ..usize::try_from(*offset + *length).unwrap()];
                    (
                        PhysicalRef {
                            allocation_id: checked.allocation_id,
                            arena,
                            offset: DecimalU64::parse(&offset.to_string()).unwrap(),
                            byte_length: DecimalU64::parse(&length.to_string()).unwrap(),
                            record_sha256: digest(frame),
                        },
                        *kind,
                    )
                })
                .collect::<Vec<_>>();
            // First access performs exactly the original hash and canonical parser.
            for (reference, kind) in &references {
                assert_eq!(
                    checked.resolve(reference, *kind).unwrap(),
                    checked.decode(reference).unwrap()
                );
            }
            assert_eq!(checked.decoded.lock().unwrap().len(), references.len());
            for (reference, kind) in &references {
                let mut changed = checked.resolve(reference, *kind).unwrap();
                changed["memo-test"] = json!(true);
                assert_ne!(checked.resolve(reference, *kind).unwrap(), changed);
                let mut bad = reference.clone();
                bad.record_sha256 = digest(b"wrong reference after warmup");
                assert!(checked.resolve(&bad, *kind).is_err());
                bad = reference.clone();
                bad.allocation_id =
                    PackageUuid::parse("97000000-0000-4000-8000-000000009999").unwrap();
                assert!(checked.resolve(&bad, *kind).is_err());
                bad = reference.clone();
                bad.byte_length = DecimalU64::parse("1").unwrap();
                assert!(checked.resolve(&bad, *kind).is_err());
                assert!(checked.resolve(reference, FrameKind::Padding).is_err());
            }
            // Same authenticated corpus and access order; no native IO or barriers.
            let started = Instant::now();
            for _ in 0..16 {
                for (reference, _) in &references {
                    black_box(checked.decode(reference).unwrap());
                }
            }
            uncached_time += started.elapsed();
            let started = Instant::now();
            for _ in 0..16 {
                for (reference, kind) in &references {
                    black_box(checked.resolve(reference, *kind).unwrap());
                }
            }
            cached_time += started.elapsed();
            resolved += references.len() * 16;
        }
        assert!(resolved > 160);
        eprintln!(
            "immutable frame memo: {resolved} identical record accesses; uncached {uncached_time:?}; cached {cached_time:?}"
        );
        // Invalid canonical input is never inserted, even with its correct digest.
        let mut malformed = encode_frame(Arena::Data, FrameKind::Semantic, b"{}", limits).unwrap();
        malformed[HEADER] = b'[';
        let id = PackageUuid::parse("97000000-0000-4000-8000-000000009999").unwrap();
        let checked = PackedAllocation::open(id, Arena::Data, &malformed, limits).unwrap();
        let reference = PhysicalRef {
            allocation_id: id,
            arena: Arena::Data,
            offset: DecimalU64::parse("0").unwrap(),
            byte_length: DecimalU64::parse(&malformed.len().to_string()).unwrap(),
            record_sha256: digest(&malformed),
        };
        for _ in 0..2 {
            assert!(checked.resolve(&reference, FrameKind::Semantic).is_err());
        }
        assert!(checked.decoded.lock().unwrap().is_empty());
    }
}
