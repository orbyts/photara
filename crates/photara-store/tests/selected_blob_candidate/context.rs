//! D19 context from fixed reference inputs, never from a candidate's selected ledger.
//! New-only construction/semantic checks; not a physical or HEAD driver.
use super::{
    keys::{JsonKey, MixedObjectKey},
    legacy::{self, Result, TrustedLegacy, ensure},
    package::{
        AllocationRegistration, Identity, PackageContext, Registration, RootSemantic,
        SemanticMembers,
    },
    provider::{RawDescription, RawMetadata},
};
use photara_core::contracts::schema::QualifiedName;
use photara_store::package::{ObjectKind, ObjectRef};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
const CORPUS: &[u8] =
    include_bytes!("../../../../docs/architecture/proposals/ps2/selected-blob/linked.json");
pub(super) struct D19Context {
    identity: Identity,
    registration: Registration,
    required_features: BTreeSet<String>,
    legacy: TrustedLegacy,
}
fn number(v: &Value) -> Result<u64> {
    let s = v.as_str().ok_or("context decimal string")?;
    let n = s.parse::<u64>().map_err(|_| "context decimal range")?;
    ensure(n.to_string() == s, "context canonical decimal")?;
    Ok(n)
}
fn uuid(v: &Value) -> Result<String> {
    let s = v.as_str().ok_or("context UUID string")?;
    let id = uuid::Uuid::parse_str(s).map_err(|_| "context UUID")?;
    ensure(
        !id.is_nil() && id.to_string() == s,
        "context canonical UUID",
    )?;
    Ok(s.into())
}
fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let m = v.as_object().ok_or("context object")?;
    ensure(
        m.len() == names.len() && names.iter().all(|k| m.contains_key(*k)),
        "context exact fields",
    )
}
fn schema(v: &Value, name: &str, version: u64, extra: &[&str], project: &str) -> Result<()> {
    let mut names = vec!["schema", "project_id", "extensions"];
    names.extend(extra);
    fields(v, &names)?;
    ensure(
        v["schema"] == json!({"id":name,"version":version}) && v["project_id"] == project,
        "context schema/project",
    )?;
    ensure(
        v["extensions"]
            .as_object()
            .is_some_and(|m| m.keys().all(|k| QualifiedName::parse(k.clone()).is_ok())),
        "context extension namespace",
    )
}
fn get(r: &Value, resolve: &mut impl FnMut(&Value) -> Result<Value>) -> Result<(JsonKey, Value)> {
    let key = MixedObjectKey::parse(r)?.json_key()?;
    let value = resolve(r)?;
    let bytes = photara_core::canonical_json(&value).map_err(|_| "context canonical JSON")?;
    legacy::parse(&bytes)?;
    ensure(
        legacy::reference(&bytes)?.sha256.as_str() == key.0 && bytes.len() as u64 == key.1,
        "context resolved reference",
    )?;
    Ok((key, value))
}
impl D19Context {
    /// Separately trusted fixed registration epoch. No candidate argument exists.
    pub(super) fn trusted() -> Result<Self> {
        Self::from_trusted(false)
    }
    pub(super) fn trusted_converted() -> Result<Self> {
        Self::from_trusted(true)
    }
    #[allow(
        clippy::too_many_lines,
        reason = "Keep the trusted original identity and registration construction in one pass"
    )]
    fn from_trusted(converted: bool) -> Result<Self> {
        let corpus_bytes: &[u8] = if converted {
            include_bytes!(
                "../../../../docs/architecture/proposals/ps2/selected-blob/converted.json"
            )
        } else {
            CORPUS
        };
        ensure(
            corpus_bytes.len() <= 8 * 1024 * 1024,
            "context corpus bound",
        )?;
        let corpus: Value =
            serde_json::from_slice(corpus_bytes).map_err(|_| "context fixed corpus")?;
        let legacy = TrustedLegacy::d19()?;
        ensure(
            corpus["original_package_sha256"] == legacy.original_package_sha256,
            "context actual legacy source",
        )?;
        let manifest = legacy::parse(&legacy.bootstrap_bytes)?;
        let mut required_features: BTreeSet<String> = manifest["required_features"]
            .as_array()
            .ok_or("context bootstrap features")?
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or("context feature string")
            })
            .collect::<Result<_>>()?;
        for feature in [
            "photara.scalable-storage.v1",
            "photara.sealed-roots.v1",
            "photara.storage-accounting.v1",
            "photara.whole-blob-storage.v1",
        ] {
            required_features.insert(feature.into());
        }
        let original = &corpus["original_evidence"];
        let profile = original["profile"]
            .as_str()
            .ok_or("context profile")?
            .to_owned();
        QualifiedName::parse(profile.clone()).map_err(|_| "context profile grammar")?;
        let mut allocations = BTreeMap::new();
        for (id, v) in original["allocations"]
            .as_object()
            .ok_or("context registrations")?
        {
            uuid(&json!(id))?;
            let layout = v["layout"].as_str().ok_or("context layout")?;
            let arena = v["arena"].as_str().ok_or("context arena")?;
            ensure(
                matches!(layout, "framed-json" | "whole-blob")
                    && matches!(arena, "data" | "metadata")
                    && (layout != "whole-blob" || arena == "data"),
                "context allocation variant",
            )?;
            let description = RawDescription {
                extent: number(&v["extent"])?,
                device: number(&v["witness"]["device"])?,
                inode: number(&v["witness"]["inode"])?,
            };
            ensure(description.inode > 0, "context original inode")?;
            allocations.insert(
                id.clone(),
                AllocationRegistration {
                    arena: arena.into(),
                    layout: layout.into(),
                    description,
                    registered_charge: number(&v["registered_charge"])?,
                },
            );
        }
        let registration = Registration {
            profile,
            incarnation: uuid(&original["incarnation"])?,
            allocations,
            standing_control: number(&original["standing_control"])?,
            directory_allowance: number(&original["directory_allowance"])?,
            retained_source: if converted {
                Some(trusted_snapshot(original)?)
            } else {
                None
            },
        };
        ensure(
            converted || number(&original["retained_directory_allowance"])? == 0,
            "context no original conversion registration",
        )?;
        let identity = Identity {
            project: legacy.project.clone(),
            library: legacy.library.clone(),
            bootstrap_sha256: legacy::hash(&legacy.bootstrap_bytes),
            manifest_bytes: legacy.bootstrap_bytes.clone(),
        };
        Ok(Self {
            identity,
            registration,
            required_features,
            legacy,
        })
    }
    pub(super) fn identity(&self) -> &Identity {
        &self.identity
    }
    pub(super) fn registration(&self) -> &Registration {
        &self.registration
    }
    pub(super) fn required_features(&self) -> &BTreeSet<String> {
        &self.required_features
    }
    /// Observed frame descriptors are supplied by the real frame loader, not by ledger fields.
    pub(super) fn observe_allocations(
        &self,
        framed: &BTreeMap<String, (String, RawDescription)>,
        raw: &dyn RawMetadata,
    ) -> Result<()> {
        let raw_ids = raw.allocation_ids()?;
        let expected_raw = self
            .registration
            .allocations
            .iter()
            .filter(|(_, r)| r.layout == "whole-blob")
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        ensure(raw_ids == expected_raw, "context exact raw allocation set")?;
        let expected_framed = self
            .registration
            .allocations
            .iter()
            .filter(|(_, r)| r.layout == "framed-json")
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        ensure(
            framed.keys().cloned().collect::<BTreeSet<_>>() == expected_framed,
            "context exact framed allocation set",
        )?;
        for (id, r) in &self.registration.allocations {
            if r.layout == "whole-blob" {
                ensure(
                    raw.describe(id)? == r.description,
                    "context original raw registration",
                )?;
            } else {
                let (arena, description) = framed.get(id).ok_or("context framed observation")?;
                ensure(
                    *arena == r.arena && *description == r.description,
                    "context original framed registration",
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn empty_index(
        &self,
        state: &Value,
        mut resolve: impl FnMut(&Value) -> Result<Value>,
    ) -> Result<BTreeSet<JsonKey>> {
        self.state_identity(state)?;
        let (index_key, index) = get(&state["operation_index"], &mut resolve)?;
        schema(
            &index,
            "photara.package.operation-index",
            2,
            &[
                "library_id",
                "bootstrap_sha256",
                "accepted",
                "by_id",
                "by_ordinal",
            ],
            &self.identity.project,
        )?;
        ensure(
            index["library_id"] == self.identity.library
                && index["bootstrap_sha256"] == self.identity.bootstrap_sha256,
            "context index identity",
        )?;
        let seed = json!({"domain":"photara.package.accepted-prefix.v1","project_id":self.identity.project,"library_id":self.identity.library,"bootstrap_sha256":self.identity.bootstrap_sha256,"through_ordinal":"0"});
        let accepted = json!({"through_ordinal":"0","prefix_sha256":legacy::hash(&photara_core::canonical_json(&seed).map_err(|_|"context prefix canonical")?)});
        ensure(
            index["accepted"] == accepted
                && state["accepted"] == accepted
                && state["journal_inclusion"].is_null(),
            "context exact zero operation state",
        )?;
        let mut members = BTreeSet::from([index_key]);
        for (field, name) in [
            ("by_id", "photara.package.operation-id-leaf"),
            ("by_ordinal", "photara.package.operation-ordinal-leaf"),
        ] {
            let (key, node) = get(&index[field], &mut resolve)?;
            schema(
                &node,
                name,
                1,
                &["count", "entries"],
                &self.identity.project,
            )?;
            ensure(
                node["count"] == "0" && node["entries"] == json!([]),
                "context canonical empty operation tree",
            )?;
            members.insert(key);
        }
        Ok(members)
    }
    fn state_identity(&self, state: &Value) -> Result<()> {
        ensure(
            state["project_id"] == self.identity.project
                && state["library_id"] == self.identity.library
                && state["bootstrap_sha256"] == self.identity.bootstrap_sha256,
            "context selected StateRoot identity",
        )?;
        ensure(
            state["authored_revision"] == self.legacy.authored_revision,
            "context preserved authored revision",
        )
    }
    pub(super) fn null_selectors(
        &self,
        root: &Value,
        state: &Value,
        ledger: &Value,
        source_count: usize,
        mut resolve: impl FnMut(&Value) -> Result<Value>,
    ) -> Result<BTreeSet<JsonKey>> {
        self.state_identity(state)?;
        ensure(
            root["project_id"] == self.identity.project
                && root["library_id"] == self.identity.library
                && root["bootstrap_sha256"] == self.identity.bootstrap_sha256,
            "context RootSet identity",
        )?;
        ensure(
            state["resource_state"].is_null()
                && !self
                    .required_features
                    .contains("photara.resource-backings.v1")
                && !self
                    .required_features
                    .contains("photara.resource-state-trees.v1"),
            "context null resource dispatch",
        )?;
        ensure(
            root["conversion_source"].is_null()
                && ledger["conversion_source"].is_null()
                && source_count == 0
                && ledger["retained_directory_allowance"] == "0",
            "context null conversion dispatch",
        )?;
        let mut members = BTreeSet::new();
        for (r, name, charge) in [
            (
                &root["pinned_roots"],
                "photara.package.retained-root-leaf",
                false,
            ),
            (
                &root["retention_evidence"],
                "photara.resource.retention-evidence-leaf",
                false,
            ),
            (
                &ledger["retained_file_charge_root"],
                "photara.package.retained-file-charge-leaf",
                true,
            ),
        ] {
            let (key, v) = get(r, &mut resolve)?;
            let extras = if charge {
                vec!["count", "entries", "charged_high_water"]
            } else {
                vec!["count", "entries"]
            };
            schema(&v, name, 1, &extras, &self.identity.project)?;
            ensure(
                v["count"] == "0"
                    && v["entries"] == json!([])
                    && (!charge || v["charged_high_water"] == "0"),
                "context canonical empty null dependency",
            )?;
            members.insert(key);
        }
        Ok(members)
    }
    pub(super) fn semantic_members(
        &self,
        state: &Value,
        mut resolve: impl FnMut(&Value) -> Result<Value>,
        mut blob_extent: impl FnMut(&ObjectRef) -> Result<u64>,
    ) -> Result<SemanticMembers> {
        ensure(
            state["resource_state"].is_null(),
            "context preserved legacy has no resource upgrade",
        )?;
        let mut members = self.legacy_members(state, &mut resolve, &mut blob_extent)?;
        members.json.extend(self.empty_index(state, &mut resolve)?);
        Ok(members)
    }
    pub(super) fn package<'a>(&'a self, raw: &'a dyn RawMetadata) -> PackageContext<'a> {
        PackageContext {
            identity: &self.identity,
            registration: &self.registration,
            required_features: &self.required_features,
            semantic: self,
            raw,
            snapshot: None,
        }
    }
    pub(super) fn package_with_snapshot<'a>(
        &'a self,
        raw: &'a dyn RawMetadata,
        snapshot: &'a dyn super::provider::SnapshotMetadata,
    ) -> PackageContext<'a> {
        let mut context = self.package(raw);
        context.snapshot = Some(snapshot);
        context
    }
}
impl RootSemantic for D19Context {
    fn legacy_members(
        &self,
        state: &Value,
        mut resolve: &mut dyn FnMut(&Value) -> Result<Value>,
        mut blob_extent: &mut dyn FnMut(&ObjectRef) -> Result<u64>,
    ) -> Result<SemanticMembers> {
        self.state_identity(state)?;
        let mut json = BTreeSet::new();
        let authored: ObjectRef = serde_json::from_value(state["authored"].clone())
            .map_err(|_| "context authored ref")?;
        let history: ObjectRef =
            serde_json::from_value(state["history"].clone()).map_err(|_| "context history ref")?;
        let closure = self.legacy.verify_semantic_metadata(
            &authored,
            &history,
            |r| {
                let rv = serde_json::to_value(r).map_err(|_| "context reference encoding")?;
                let (_, v) = get(&rv, &mut resolve)?;
                photara_core::canonical_json(&v).map_err(|_| "context semantic canonical")
            },
            &mut blob_extent,
        )?;
        let mut blobs = BTreeSet::new();
        for r in closure {
            let key = MixedObjectKey::parse(
                &serde_json::to_value(&r).map_err(|_| "context semantic reference")?,
            )?;
            match r.kind {
                ObjectKind::Json => {
                    json.insert(key.json_key()?);
                }
                ObjectKind::Blob => {
                    blobs.insert(key);
                }
            }
        }
        Ok(SemanticMembers { json, blobs })
    }
}

fn trusted_snapshot(original: &Value) -> Result<super::package::SnapshotRegistration> {
    use super::{
        package::{SnapshotFileRegistration, SnapshotRegistration},
        provider::SnapshotDescription,
    };
    let descriptor = original["conversion"]
        .as_str()
        .ok_or("original conversion bytes")?
        .as_bytes()
        .to_vec();
    let value = legacy::parse(&descriptor)?;
    let archive: Value = serde_json::from_slice(include_bytes!(
        "../../../../docs/fixtures/generation-two/d19-package-specimen.json"
    ))
    .map_err(|_| "original D19 archive")?;
    let originals: BTreeMap<Vec<String>, Vec<u8>> = archive["files"]
        .as_array()
        .ok_or("original files")?
        .iter()
        .map(|r| {
            Ok((
                r["path"]
                    .as_str()
                    .ok_or("original path")?
                    .split('/')
                    .map(str::to_owned)
                    .collect(),
                r["utf8"]
                    .as_str()
                    .ok_or("original file")?
                    .as_bytes()
                    .to_vec(),
            ))
        })
        .collect::<Result<_>>()?;
    let manifest = legacy::parse(
        originals
            .get(&vec!["manifest.json".into()])
            .ok_or("original manifest")?,
    )?;
    let head_bytes = originals
        .get(&vec!["HEAD.json".into()])
        .ok_or("original HEAD")?;
    let head = legacy::parse(head_bytes)?;
    let files: Vec<Value> = originals.iter().map(|(p,b)| json!({"components":p,"byte_length":b.len().to_string(),"sha256":legacy::hash(b)})).collect();
    ensure(
        value["files"] == json!(files)
            && value["project_id"] == manifest["project_id"]
            && value["source_format_version"] == manifest["format_version"]
            && value["source_head_sha256"] == legacy::hash(head_bytes)
            && value["source_commit_id"] == head["commit_id"]
            && value["source_bootstrap_sha256"]
                == legacy::hash(
                    &photara_core::canonical_json(&manifest)
                        .map_err(|_| "original manifest canonical")?,
                ),
        "original D19 conversion commitments",
    )?;
    let mut registered = BTreeMap::new();
    for (name, r) in original["retained_files"]
        .as_object()
        .ok_or("original snapshot registration")?
    {
        let path = name.split('/').map(str::to_owned).collect::<Vec<_>>();
        photara_core::contracts::resource::RelativeComponents::new(path.clone())
            .map_err(|_| "original portable snapshot path")?;
        let bytes = originals.get(&path).ok_or("original snapshot path")?;
        ensure(
            r["regular"] == true
                && number(&r["extent"])? == bytes.len() as u64
                && r["sha256"] == legacy::hash(bytes),
            "original snapshot description",
        )?;
        let physical = RawDescription {
            extent: number(&r["extent"])?,
            device: number(&r["witness"]["device"])?,
            inode: number(&r["witness"]["inode"])?,
        };
        ensure(physical.inode > 0, "original snapshot inode")?;
        registered.insert(
            path,
            SnapshotFileRegistration {
                description: SnapshotDescription {
                    regular: true,
                    physical,
                },
                sha256: r["sha256"].as_str().ok_or("original file digest")?.into(),
                registered_charge: number(&r["registered_charge"])?,
            },
        );
    }
    ensure(
        registered.keys().eq(originals.keys()),
        "original snapshot exact registration paths",
    )?;
    Ok(SnapshotRegistration {
        descriptor,
        files: registered,
        directory_allowance: number(&original["retained_directory_allowance"])?,
    })
}
