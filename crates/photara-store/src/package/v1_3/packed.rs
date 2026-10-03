//! Exact frozen PS2 framing and direct physical coordinates.
use std::collections::BTreeMap;

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
/// an accessed record; full retained-closure auditing is a separate operation.
pub struct PackedAllocation<'a> {
    allocation_id: PackageUuid,
    arena: Arena,
    bytes: &'a [u8],
    frames: BTreeMap<u64, (u64, FrameKind)>,
    limits: FrameLimits,
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
