//! Trusted original D19 closure, established once by the actual legacy reader.
//! Structural adapter stores no original media bytes and performs no media I/O.
use photara_store::package::{
    self, Inventory, JsonLimits, MemoryPackage, ObjectKind, ObjectRef, PackageLimits,
    parse_canonical_json,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Result<T> = std::result::Result<T, &'static str>;
pub(super) fn ensure(ok: bool, error: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(error) }
}
pub(super) fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn parse(bytes: &[u8]) -> Result<Value> {
    parse_canonical_json(
        bytes,
        JsonLimits {
            max_bytes: 1024 * 1024,
            max_depth: 64,
            max_members: 4096,
            max_array_elements: 4096,
        },
    )
    .map_err(|_| "bounded canonical metadata")
}
pub(super) fn reference(bytes: &[u8]) -> Result<ObjectRef> {
    serde_json::from_value(serde_json::json!({"kind":"json","sha256":hash(bytes),"byte_length":bytes.len().to_string()})).map_err(|_| "JSON ObjectRef")
}
pub(super) struct TrustedLegacy {
    pub(super) project: String,
    pub(super) library: String,
    pub(super) bootstrap_bytes: Vec<u8>,
    pub(super) authored: ObjectRef,
    pub(super) history: ObjectRef,
    pub(super) authored_revision: String,
    pub(super) closure: BTreeSet<ObjectRef>,
    pub(super) json: BTreeMap<ObjectRef, Vec<u8>>,
    pub(super) blobs: BTreeSet<ObjectRef>,
    pub(super) original_package_sha256: String,
    pub(super) setup_verified_blobs: usize,
}
impl TrustedLegacy {
    /// Strong verification of fixed immutable *reference input*, not current allocations.
    /// Call before structural-open instrumentation; discard reference raw media afterward.
    pub(super) fn d19() -> Result<Self> {
        let archive_bytes =
            include_bytes!("../../../../docs/fixtures/generation-two/d19-package-specimen.json");
        let archive: Value =
            serde_json::from_slice(archive_bytes).map_err(|_| "D19 reference archive")?;
        let rows = archive["files"].as_array().ok_or("D19 files")?;
        ensure(rows.len() <= 256, "D19 file count")?;
        let mut files = BTreeMap::new();
        for row in rows {
            let path = row["path"].as_str().ok_or("D19 path")?;
            let bytes = row["utf8"]
                .as_str()
                .ok_or("D19 file bytes")?
                .as_bytes()
                .to_vec();
            ensure(
                hash(&bytes) == row["sha256"].as_str().ok_or("D19 hash")?,
                "D19 exact archived bytes",
            )?;
            ensure(
                files.insert(path.to_owned(), bytes).is_none(),
                "D19 duplicate path",
            )?;
        }
        let package = MemoryPackage::new(files.clone(), PackageLimits::default())
            .map_err(|_| "D19 memory package")?;
        let validated =
            package::v1_1::validate_memory(&package).map_err(|_| "actual legacy reader")?;
        let commit = validated.commits.first().ok_or("D19 current commit")?;
        let inventory: Inventory = serde_json::from_slice(
            files
                .get(&commit.inventory.path())
                .ok_or("D19 original inventory")?,
        )
        .map_err(|_| "D19 inventory")?;
        let closure: BTreeSet<_> = inventory.objects.into_iter().collect();
        let mut json = BTreeMap::new();
        let mut blobs = BTreeSet::new();
        for r in &closure {
            match r.kind {
                ObjectKind::Blob => {
                    blobs.insert(r.clone());
                }
                ObjectKind::Json => {
                    let object = validated
                        .objects
                        .get(&r.sha256)
                        .ok_or("D19 typed JSON closure")?;
                    ensure(
                        object.reference == *r
                            && files.get(&r.path()) == Some(&object.canonical_bytes),
                        "D19 exact selected JSON",
                    )?;
                    json.insert(r.clone(), object.canonical_bytes.clone());
                }
            }
        }
        ensure(
            validated.verified_blobs == 1 && blobs.len() == 1,
            "D19 actual managed Blob",
        )?;
        Ok(Self {
            project: validated.bootstrap.project_id.to_string(),
            library: validated.authored.value["owning_library_id"]
                .as_str()
                .ok_or("D19 library")?
                .into(),
            bootstrap_bytes: files.get("manifest.json").ok_or("D19 bootstrap")?.clone(),
            authored: commit.authored.clone(),
            history: commit.history.clone(),
            authored_revision: validated.authored.value["authored_revision"]
                .as_str()
                .ok_or("D19 authored revision")?
                .into(),
            closure,
            json,
            blobs,
            original_package_sha256: hash(archive_bytes),
            setup_verified_blobs: validated.verified_blobs,
        })
    }
    /// Snapshot-preservation adapter, not a universal arbitrary legacy schema reader.
    /// Blob callback receives `ObjectRefs` only and must return structural extent metadata.
    pub(super) fn verify_semantic_metadata(
        &self,
        authored: &ObjectRef,
        history: &ObjectRef,
        mut json: impl FnMut(&ObjectRef) -> Result<Vec<u8>>,
        mut blob_extent: impl FnMut(&ObjectRef) -> Result<u64>,
    ) -> Result<BTreeSet<ObjectRef>> {
        ensure(
            authored == &self.authored && history == &self.history,
            "D19 exact authored/history selection",
        )?;
        for (r, original) in &self.json {
            let current = json(r)?;
            parse(&current)?;
            ensure(
                reference(&current)? == *r && current == *original,
                "D19 selected legacy bytes changed",
            )?;
        }
        for r in &self.blobs {
            ensure(
                blob_extent(r)? == r.byte_length.get(),
                "D19 selected Blob extent",
            )?;
        }
        Ok(self.closure.clone())
    }
}
