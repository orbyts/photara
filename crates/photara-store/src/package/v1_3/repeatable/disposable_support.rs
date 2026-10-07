#![allow(
    dead_code,
    clippy::wildcard_imports,
    reason = "Shared existing-corpus test support"
)]
//! Internal registration model for the controlled disposable fixture only.
use crate::package::v1_3::*;
use crate::package::*;
use serde_json::Value;
use std::collections::BTreeMap;
pub fn bytes(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
pub fn hash(b: &[u8]) -> Sha256Hex {
    use sha2::{Digest, Sha256};
    Sha256Hex::parse(&format!("{:x}", Sha256::digest(b))).unwrap()
}
#[cfg(test)]
pub fn unhex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect()
}
pub fn tree() -> TreeLimits {
    TreeLimits {
        max_depth: 32,
        max_pages: 1024,
        max_entries: 16384,
        max_leaf_entries: 64,
        max_branch_children: 16,
    }
}
pub fn limits() -> ReaderLimits {
    let json = JsonLimits {
        max_bytes: 1024 * 1024,
        max_depth: 64,
        max_members: 4096,
        max_array_elements: 16384,
    };
    ReaderLimits {
        json,
        tree: tree(),
        semantic: PackageLimits::default(),
        resources: ResourceLimits {
            tree: tree(),
            max_objects: 16384,
            max_record_bytes: 1024 * 1024,
        },
        accounting: AccountingLimits {
            tree: tree(),
            max_objects: 16384,
            max_record_bytes: 1024 * 1024,
            max_tips: 64,
            max_path_components: 64,
        },
        max_roles: 64,
        max_prefix_bytes: 16 * 1024 * 1024,
    }
}
pub struct Allocation {
    pub bytes: Vec<u8>,
    pub arena: Arena,
    pub layout: AllocationLayout,
    pub device: u64,
    pub inode: u64,
    pub charge: u64,
}
pub struct Fixture {
    pub data: Value,
    pub allocations: BTreeMap<PackageUuid, Allocation>,
    pub loose: BTreeMap<ObjectRef, Vec<u8>>,
    pub registration: SettledRegistration,
}
impl Fixture {
    #[cfg(test)]
    pub fn load(name: &str) -> Self {
        let raw = match name {
            "integrated" => {
                include_str!(
                    "../../../../../../docs/architecture/proposals/ps2/integrated/linked.json"
                )
            }
            "blob" => {
                include_str!(
                    "../../../../../../docs/architecture/proposals/ps2/selected-blob/linked.json"
                )
            }
            _ => panic!(),
        };
        let data: Value = serde_json::from_str(raw).unwrap();
        Self::from_value(data).expect("fixed test registration")
    }
    /// Bounded parsing of the independently captured disposable registration.
    #[expect(clippy::too_many_lines, reason = "Single bounded registration decoder")]
    pub fn from_value(data: Value) -> std::io::Result<Self> {
        fn obj(v: &Value) -> std::io::Result<&serde_json::Map<String, Value>> {
            v.as_object()
                .ok_or_else(|| std::io::ErrorKind::InvalidData.into())
        }
        let fail = || std::io::Error::from(std::io::ErrorKind::InvalidData);
        let decimal = |v: &Value| -> std::io::Result<u64> {
            let s = v.as_str().ok_or_else(fail)?;
            let n = s.parse::<u64>().map_err(|_| fail())?;
            if n.to_string() != s {
                return Err(fail());
            }
            Ok(n)
        };
        let boot = &data["bootstrap"];
        for name in ["manifest", "head", "commit"] {
            let raw = boot[name].as_str().ok_or_else(fail)?;
            let parsed = crate::package::parse_canonical_json(raw.as_bytes(), limits().json)
                .map_err(|_| fail())?;
            if !parsed.is_object() {
                return Err(fail());
            }
        }
        let manifest: Value = serde_json::from_str(boot["manifest"].as_str().ok_or_else(fail)?)
            .map_err(|_| fail())?;
        let commit: Value =
            serde_json::from_str(boot["commit"].as_str().ok_or_else(fail)?).map_err(|_| fail())?;
        PackageUuid::parse(manifest["project_id"].as_str().ok_or_else(fail)?)
            .map_err(|_| fail())?;
        PackageUuid::parse(commit["root_set"]["library_id"].as_str().ok_or_else(fail)?)
            .map_err(|_| fail())?;
        let controls = obj(&data["loose"])?;
        if controls.len() > 128 {
            return Err(fail());
        }
        let mut loose = BTreeMap::new();
        let mut ledger = None;
        for value in controls.values() {
            let b = value.as_str().ok_or_else(fail)?.as_bytes().to_vec();
            let v = crate::package::parse_canonical_json(&b, limits().json).map_err(|_| fail())?;
            if v["schema"]["id"] == "photara.storage.ledger" && ledger.replace(v).is_some() {
                return Err(fail());
            }
            let reference = ObjectRef {
                kind: ObjectKind::Json,
                sha256: hash(&b),
                byte_length: DecimalU64::parse(&b.len().to_string()).map_err(|_| fail())?,
            };
            if loose.insert(reference, b).is_some() {
                return Err(fail());
            }
        }
        let ledger = ledger.ok_or_else(fail)?;
        let entries = obj(&data["allocations"])?;
        if entries.len() != 8 {
            return Err(fail());
        }
        let mut allocations = BTreeMap::new();
        let mut total = 0usize;
        for (id, a) in entries {
            let id = PackageUuid::parse(id).map_err(|_| fail())?;
            let b = decode_hex(a["hex"].as_str().ok_or_else(fail)?)?;
            total = total
                .checked_add(b.len())
                .filter(|n| *n <= 64 << 20)
                .ok_or_else(fail)?;
            if !a["layout"].is_null() && a["layout"] != "framed-json" {
                return Err(fail());
            }
            let charge = (b.len() as u64).max(1).div_ceil(4096) * 4096;
            let device = decimal(&a["witness"]["device"])?;
            let inode = decimal(&a["witness"]["inode"])?;
            if inode == 0 {
                return Err(fail());
            }
            allocations.insert(
                id,
                Allocation {
                    bytes: b,
                    arena: serde_json::from_value(a["arena"].clone()).map_err(|_| fail())?,
                    layout: AllocationLayout::FramedJson,
                    device,
                    inode,
                    charge,
                },
            );
        }
        let files = obj(&data["source_files"])?;
        if files.len() > 256 {
            return Err(fail());
        }
        let mut retained_files = BTreeMap::new();
        for (path, b) in files {
            let path_parts = path.split('/').map(str::to_owned).collect::<Vec<_>>();
            photara_core::contracts::resource::RelativeComponents::new(path_parts.clone())
                .map_err(|_| fail())?;
            let raw = decode_hex(b.as_str().ok_or_else(fail)?)?;
            total = total
                .checked_add(raw.len())
                .filter(|n| *n <= 64 << 20)
                .ok_or_else(fail)?;
            let w = &data["source_witnesses"][path];
            let inode = decimal(&w["inode"])?;
            if inode == 0 {
                return Err(fail());
            }
            retained_files.insert(
                path_parts,
                RetainedObservation {
                    extent: raw.len() as u64,
                    sha256: hash(&raw),
                    registered_charge: (raw.len() as u64).max(1).div_ceil(4096) * 4096,
                    device: decimal(&w["device"])?,
                    inode,
                },
            );
        }
        let registration = SettledRegistration {
            profile: ledger["profile"].as_str().ok_or_else(fail)?.into(),
            incarnation: PackageUuid::parse(ledger["incarnation"].as_str().ok_or_else(fail)?)
                .map_err(|_| fail())?,
            allocations: allocations.keys().copied().collect(),
            conversion: if ledger["conversion_source"].is_null() {
                None
            } else {
                Some(
                    serde_json::from_value(ledger["conversion_source"].clone())
                        .map_err(|_| fail())?,
                )
            },
            retained_files,
            standing_control: decimal(&ledger["standing_control"])?,
            charge_unit: 4096,
            directory_allowance: decimal(&ledger["directory_allowance"])?,
            retained_directory_allowance: decimal(&ledger["retained_directory_allowance"])?,
        };
        Ok(Self {
            data,
            allocations,
            loose,
            registration,
        })
    }
}
impl AllocationInspection for Fixture {
    fn observe(&self, id: PackageUuid) -> Result<AllocationObservation, PackageError> {
        let a = self.allocations.get(&id).ok_or(PackageError::Integrity)?;
        Ok(AllocationObservation {
            arena: a.arena,
            layout: a.layout,
            extent: a.bytes.len() as u64,
            registered_charge: a.charge,
            device: a.device,
            inode: a.inode,
        })
    }
    fn framed_prefix_digest(
        &self,
        id: PackageUuid,
        n: u64,
        max: u64,
    ) -> Result<Sha256Hex, PackageError> {
        let a = self.allocations.get(&id).ok_or(PackageError::Integrity)?;
        if n > max || a.layout != AllocationLayout::FramedJson {
            return Err(PackageError::Limit);
        }
        Ok(hash(
            a.bytes
                .get(..usize::try_from(n).map_err(|_| PackageError::Limit)?)
                .ok_or(PackageError::Integrity)?,
        ))
    }
}

fn decode_hex(s: &str) -> std::io::Result<Vec<u8>> {
    let fail = || std::io::Error::from(std::io::ErrorKind::InvalidData);
    if s.len() > 32 << 20
        || !s.len().is_multiple_of(2)
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(fail());
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|v| {
            u8::from_str_radix(std::str::from_utf8(v).map_err(|_| fail())?, 16).map_err(|_| fail())
        })
        .collect()
}
