#![allow(
    dead_code,
    clippy::wildcard_imports,
    reason = "Shared existing-corpus test support"
)]
//! Production reader composition against already-frozen canonical packages.
use super::photara_store::package::v1_3::*;
use super::photara_store::package::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub fn bytes(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
pub fn hash(b: &[u8]) -> Sha256Hex {
    use sha2::{Digest, Sha256};
    Sha256Hex::parse(&format!("{:x}", Sha256::digest(b))).unwrap()
}
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
    pub fn load(name: &str) -> Self {
        let raw = match name {
            "integrated" => {
                include_str!("../../../../docs/architecture/proposals/ps2/integrated/linked.json")
            }
            "blob" => {
                include_str!(
                    "../../../../docs/architecture/proposals/ps2/selected-blob/linked.json"
                )
            }
            _ => panic!(),
        };
        let data: Value = serde_json::from_str(raw).unwrap();
        Self::from_value(data)
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Capture independent synthetic fixture registration once"
    )]
    pub fn from_value(data: Value) -> Self {
        let loose: BTreeMap<_, _> = data["loose"]
            .as_object()
            .unwrap()
            .values()
            .map(|s| {
                let b = s.as_str().unwrap().as_bytes().to_vec();
                (
                    ObjectRef {
                        kind: ObjectKind::Json,
                        sha256: hash(&b),
                        byte_length: DecimalU64::parse(&b.len().to_string()).unwrap(),
                    },
                    b,
                )
            })
            .collect();
        let ledger: Value = loose
            .values()
            .map(|b| serde_json::from_slice::<Value>(b).unwrap())
            .find(|v| v["schema"]["id"] == "photara.storage.ledger")
            .unwrap();
        // Freeze synthetic original registration before any adversarial mutation.
        let allocations: BTreeMap<_, _> = data["allocations"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(id, a)| {
                let b = unhex(a["hex"].as_str().unwrap());
                let charge = (b.len() as u64).max(1).div_ceil(4096) * 4096;
                (
                    PackageUuid::parse(id).unwrap(),
                    Allocation {
                        bytes: b,
                        arena: serde_json::from_value(a["arena"].clone()).unwrap(),
                        layout: if a["layout"] == "whole-blob" {
                            AllocationLayout::WholeBlob
                        } else {
                            AllocationLayout::FramedJson
                        },
                        device: a["witness"]["device"].as_str().unwrap().parse().unwrap(),
                        inode: a["witness"]["inode"].as_str().unwrap().parse().unwrap(),
                        charge,
                    },
                )
            })
            .collect();
        let retained_files = data["source_files"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(path, b)| {
                let raw = unhex(b.as_str().unwrap());
                let w = &data["source_witnesses"][path];
                (
                    path.split('/').map(str::to_owned).collect(),
                    RetainedObservation {
                        extent: raw.len() as u64,
                        sha256: hash(&raw),
                        registered_charge: (raw.len() as u64).max(1).div_ceil(4096) * 4096,
                        device: w["device"].as_str().unwrap().parse().unwrap(),
                        inode: w["inode"].as_str().unwrap().parse().unwrap(),
                    },
                )
            })
            .collect();
        let n = |k: &str| ledger[k].as_str().unwrap().parse::<u64>().unwrap();
        let registration = SettledRegistration {
            profile: ledger["profile"].as_str().unwrap().into(),
            incarnation: PackageUuid::parse(ledger["incarnation"].as_str().unwrap()).unwrap(),
            allocations: allocations.keys().copied().collect(),
            conversion: if ledger["conversion_source"].is_null() {
                None
            } else {
                Some(serde_json::from_value(ledger["conversion_source"].clone()).unwrap())
            },
            retained_files,
            standing_control: n("standing_control"),
            charge_unit: 4096,
            directory_allowance: n("directory_allowance"),
            retained_directory_allowance: n("retained_directory_allowance"),
        };
        Self {
            data,
            allocations,
            loose,
            registration,
        }
    }
    pub fn envelope(&self) -> SelectedEnvelope {
        let boot = &self.data["bootstrap"];
        let commit: Value = serde_json::from_str(boot["commit"].as_str().unwrap()).unwrap();
        SelectedEnvelope::parse(
            boot["manifest"].as_str().unwrap().as_bytes(),
            boot["head"].as_str().unwrap().as_bytes(),
            boot["commit"].as_str().unwrap().as_bytes(),
            SelectionIdentity {
                project: PackageUuid::parse(commit["project_id"].as_str().unwrap()).unwrap(),
                library: PackageUuid::parse(commit["root_set"]["library_id"].as_str().unwrap())
                    .unwrap(),
                bootstrap_sha256: Sha256Hex::parse(commit["bootstrap_sha256"].as_str().unwrap())
                    .unwrap(),
            },
            limits().json,
        )
        .unwrap()
    }
    pub fn verify(&self) -> Result<SettledPackage, PackageError> {
        inspect_settled(
            &self.envelope(),
            self,
            self,
            self,
            &self.registration,
            &limits(),
        )
    }
}
impl PhysicalRecords for Fixture {
    fn resolve(&self, r: &PhysicalRef, k: FrameKind) -> Result<Value, PackageError> {
        let a = self
            .allocations
            .get(&r.allocation_id)
            .ok_or(PackageError::Integrity)?;
        PackedAllocation::open(
            r.allocation_id,
            a.arena,
            &a.bytes,
            FrameLimits {
                json: limits().json,
                max_allocation_bytes: 16 * 1024 * 1024,
                max_frames: 65536,
            },
        )?
        .resolve(r, k)
    }
}
impl ControlRecords for Fixture {
    fn controls(&self) -> Result<BTreeSet<ObjectRef>, PackageError> {
        Ok(self.loose.keys().cloned().collect())
    }
    fn read(&self, r: &ObjectRef) -> Result<Vec<u8>, PackageError> {
        self.loose.get(r).cloned().ok_or(PackageError::Integrity)
    }
}
impl BlobMetadata for Fixture {
    fn extent(&self, id: PackageUuid) -> Result<u64, PackageError> {
        Ok(self
            .allocations
            .get(&id)
            .ok_or(PackageError::Integrity)?
            .bytes
            .len() as u64)
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
