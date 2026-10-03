//! Authentication of the frozen outer selection and scoped active/recovery roots.
//! This intentionally returns structural evidence, never audited/writeable evidence.
use super::super::{
    JsonLimits, ObjectKind, ObjectRef, PackageError, PackageUuid, Sha256Hex, digest,
    parse_canonical_json,
};
use super::tree::{fields, number};
use super::{
    Locator, Membership, PhysicalRecords, PhysicalRef, PhysicalTree, PhysicalTreeKind, TreeLimits,
};
use photara_core::contracts::schema::QualifiedName;
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Trusted expected package identity. Fields are comparison inputs, not authority.
pub struct SelectionIdentity {
    pub project: PackageUuid,
    pub library: PackageUuid,
    pub bootstrap_sha256: Sha256Hex,
}
/// A checksum-authenticated selection envelope. Its transitive closure has not
/// been audited. Keeping this distinct prevents structural inspection from
/// minting writable admission.
pub struct SelectedEnvelope {
    pub(super) commit: Value,
    pub(super) root: Value,
    pub(super) identity: SelectionIdentity,
    pub(super) outer_bytes: u64,
    pub(super) outer_lengths: [u64; 3],
}
#[derive(Clone, Copy, Debug)]
pub enum SelectedRole {
    Active,
    Recovery,
}
/// Exact selected state and its own locator. Proof concerns this accessed role;
/// missing active-only dependencies cannot invalidate independent recovery reads.
pub struct StructuralState {
    pub reference: ObjectRef,
    pub state: Value,
    pub locator: PhysicalRef,
    pub ownership: ObjectRef,
}

impl SelectedEnvelope {
    /// Authenticates canonical HEAD/commit/bootstrap and frozen floor/capability
    /// dispatch. Ancestor, semantic, ownership and accounting closure remain
    /// separate validations; success is never a Saved or writable result.
    /// # Errors
    /// Refuses wrong identity/hash, unsupported schemas/features/floor, malformed
    /// exact fields, noncanonical counters and caller parsing limits.
    #[expect(
        clippy::too_many_lines,
        reason = "Keep exact frozen field and identity checks in auditable protocol order"
    )]
    pub fn parse(
        manifest: &[u8],
        head: &[u8],
        commit: &[u8],
        identity: SelectionIdentity,
        limits: JsonLimits,
    ) -> Result<Self, PackageError> {
        if identity.project.as_uuid().is_nil() || identity.library.as_uuid().is_nil() {
            return Err(PackageError::Record);
        }
        let outer_lengths = [
            manifest.len() as u64,
            head.len() as u64,
            commit.len() as u64,
        ];
        let outer_bytes = manifest
            .len()
            .checked_add(head.len())
            .and_then(|n| n.checked_add(commit.len()))
            .ok_or(PackageError::Limit)? as u64;
        let manifest = parse_canonical_json(manifest, limits)?;
        let head = parse_canonical_json(head, limits)?;
        let commit = parse_canonical_json(commit, limits)?;
        fields(
            &head,
            &["schema", "project_id", "commit_id", "commit_sha256"],
        )?;
        if head["schema"] != json!({"id":"photara.package.head","version":1})
            || head["project_id"] != identity.project.to_string()
            || head["commit_id"] != commit["commit_id"]
            || head["commit_sha256"] != hash_value(&commit)?.as_str()
        {
            return Err(PackageError::Integrity);
        }
        schema(
            &commit,
            "photara.package.commit",
            1,
            &[
                "commit_id",
                "package_revision",
                "bootstrap_sha256",
                "parent",
                "write_id",
                "created_at",
                "minimum_reader",
                "required_features",
                "authored",
                "history",
                "inventory",
                "root_set",
            ],
            identity.project,
        )?;
        nonnil(&commit["commit_id"])?;
        nonnil(&commit["write_id"])?;
        if number(&commit["package_revision"])? == 0 {
            return Err(PackageError::Record);
        }
        if !commit["parent"].is_null() {
            fields(&commit["parent"], &["commit_id", "commit_sha256"])?;
            nonnil(&commit["parent"]["commit_id"])?;
            sha(&commit["parent"]["commit_sha256"])?;
        } else if number(&commit["package_revision"])? != 1 {
            return Err(PackageError::Integrity);
        }
        photara_core::context::value::Timestamp::try_from(
            commit["created_at"]
                .as_str()
                .ok_or(PackageError::Record)?
                .to_owned(),
        )
        .map_err(|_| PackageError::Record)?;
        if commit["minimum_reader"] != json!({"major":1,"minor":3}) {
            return Err(PackageError::UnsupportedVersion);
        }
        if manifest["project_id"] != identity.project.to_string()
            || hash_value(&manifest)? != identity.bootstrap_sha256
            || commit["bootstrap_sha256"] != identity.bootstrap_sha256.as_str()
        {
            return Err(PackageError::Integrity);
        }
        let inherited = features(&manifest["required_features"])?;
        let required = features(&commit["required_features"])?;
        if !inherited.is_subset(&required)
            || [
                "photara.scalable-storage.v1",
                "photara.sealed-roots.v1",
                "photara.storage-accounting.v1",
            ]
            .iter()
            .any(|f| !required.contains(*f))
        {
            return Err(PackageError::UnsupportedFeature);
        }
        for field in ["authored", "history", "inventory"] {
            json_ref(&commit[field])?;
        }
        let root = commit["root_set"].clone();
        schema(
            &root,
            "photara.package.root-set",
            2,
            &[
                "library_id",
                "bootstrap_sha256",
                "kind",
                "active",
                "recovery",
                "pinned_roots",
                "operation_index",
                "conversion_source",
                "retention_evidence",
                "inventory",
                "placement",
            ],
            identity.project,
        )?;
        if root["library_id"] != identity.library.to_string()
            || root["bootstrap_sha256"] != identity.bootstrap_sha256.as_str()
            || root["kind"] != "sealed"
        {
            return Err(PackageError::Integrity);
        }
        for field in [
            "active",
            "recovery",
            "pinned_roots",
            "operation_index",
            "retention_evidence",
            "inventory",
        ] {
            json_ref(&root[field])?;
        }
        if !root["conversion_source"].is_null() {
            json_ref(&root["conversion_source"])?;
        }
        fields(
            &root["placement"],
            &["generation", "root_placements", "accounting"],
        )?;
        if number(&root["placement"]["generation"])? == 0 {
            return Err(PackageError::Record);
        }
        physical(&root["placement"]["root_placements"])?;
        json_ref(&root["placement"]["accounting"])?;
        Ok(Self {
            commit,
            root,
            identity,
            outer_bytes,
            outer_lengths,
        })
    }
    /// Authenticates the selected role through its own placement/locator. It
    /// reads neither the other role nor Blob payloads, and proves no global total.
    /// # Errors
    /// Refuses omitted role placement, wrong identity, wrong state schema or
    /// locator/hash/tag failures. Budgets are explicitly supplied by the caller.
    pub fn open_role(
        &self,
        provider: &impl PhysicalRecords,
        role: SelectedRole,
        limits: TreeLimits,
    ) -> Result<StructuralState, PackageError> {
        let field = match role {
            SelectedRole::Active => "active",
            SelectedRole::Recovery => "recovery",
        };
        let reference = json_ref(&self.root[field])?;
        self.open_reference(provider, &reference, limits)
    }
    /// Opens an exact retained `StateRoot`. Package orchestration separately proves
    /// that this reference is selected by an active/recovery role or retained pin.
    #[expect(clippy::too_many_lines, reason = "Exact frozen StateRoot dispatch")]
    pub(super) fn open_reference(
        &self,
        provider: &impl PhysicalRecords,
        reference: &ObjectRef,
        limits: TreeLimits,
    ) -> Result<StructuralState, PackageError> {
        let reference = reference.clone();
        let root = physical(&self.root["placement"]["root_placements"])?;
        let tree = PhysicalTree::new(
            provider,
            self.identity.project,
            PhysicalTreeKind::RootPlacement,
            limits,
        );
        let placement = tree
            .lookup(&root, &reference)?
            .ok_or(PackageError::Integrity)?;
        fields(&placement, &["root", "root_id", "locator", "ownership"])?;
        nonnil(&placement["root_id"])?;
        if json_ref(&placement["root"])? != reference {
            return Err(PackageError::Integrity);
        }
        let locator = physical(&placement["locator"])?;
        let ownership = json_ref(&placement["ownership"])?;
        let resolver = Locator::new(provider, self.identity.project, locator.clone(), limits);
        let state = resolver.json(&reference, Membership::Semantic)?;
        schema(
            &state,
            "photara.package.state-root",
            2,
            &[
                "library_id",
                "bootstrap_sha256",
                "root_id",
                "authored_revision",
                "authored",
                "history",
                "resource_state",
                "operation_index",
                "accepted",
                "journal_inclusion",
                "predecessor",
                "inventory",
            ],
            self.identity.project,
        )?;
        if state["library_id"] != self.identity.library.to_string()
            || state["bootstrap_sha256"] != self.identity.bootstrap_sha256.as_str()
            || state["root_id"] != placement["root_id"]
        {
            return Err(PackageError::Integrity);
        }
        number(&state["authored_revision"])?;
        for field in ["authored", "history", "operation_index", "inventory"] {
            json_ref(&state[field])?;
        }
        let required = features(&self.commit["required_features"])?;
        if state["resource_state"].is_null() {
            if !required.contains("photara.whole-blob-storage.v1") {
                return Err(PackageError::UnsupportedFeature);
            }
        } else {
            json_ref(&state["resource_state"])?;
            for feature in [
                "photara.canonical-json.v1",
                "photara.resource-backings.v1",
                "photara.resource-state-trees.v1",
            ] {
                if !required.contains(feature) {
                    return Err(PackageError::UnsupportedFeature);
                }
            }
        }
        fields(&state["accepted"], &["through_ordinal", "prefix_sha256"])?;
        let accepted = number(&state["accepted"]["through_ordinal"])?;
        sha(&state["accepted"]["prefix_sha256"])?;
        if accepted == 0 {
            if !state["journal_inclusion"].is_null() {
                return Err(PackageError::Record);
            }
        } else {
            let journal = &state["journal_inclusion"];
            fields(
                journal,
                &[
                    "journal_id",
                    "through_sequence",
                    "prefix_sha256",
                    "resulting_authored_revision",
                    "resulting_authored_sha256",
                ],
            )?;
            nonnil(&journal["journal_id"])?;
            number(&journal["through_sequence"])?;
            sha(&journal["prefix_sha256"])?;
            if journal["resulting_authored_revision"] != state["authored_revision"]
                || journal["resulting_authored_sha256"] != state["authored"]["sha256"]
            {
                return Err(PackageError::Integrity);
            }
        }
        if !state["predecessor"].is_null() {
            fields(&state["predecessor"], &["root_sha256", "authored_revision"])?;
            sha(&state["predecessor"]["root_sha256"])?;
            number(&state["predecessor"]["authored_revision"])?;
        }
        if reference == json_ref(&self.root["active"])?
            && (state["authored"] != self.commit["authored"]
                || state["history"] != self.commit["history"]
                || state["inventory"] != self.commit["inventory"]
                || state["operation_index"] != self.root["operation_index"])
        {
            return Err(PackageError::Integrity);
        }
        Ok(StructuralState {
            reference,
            state,
            locator,
            ownership,
        })
    }
}
pub(super) fn hash_value(value: &Value) -> Result<Sha256Hex, PackageError> {
    Ok(digest(
        &photara_core::canonical_json(value).map_err(|_| PackageError::Record)?,
    ))
}
fn features(value: &Value) -> Result<BTreeSet<String>, PackageError> {
    let array = value.as_array().ok_or(PackageError::Record)?;
    let mut result = BTreeSet::new();
    let mut previous = None;
    for value in array {
        let text = value.as_str().ok_or(PackageError::Record)?;
        if !matches!(
            text,
            "photara.history.v1"
                | "photara.immutable-objects.v1"
                | "photara.library-project.v1"
                | "photara.resources.v2"
                | "photara.asset-set.v2"
                | "photara.context.v1"
                | "photara.node-contract.v2"
                | "photara.canonical-json.v1"
                | "photara.resource-backings.v1"
                | "photara.resource-state-trees.v1"
                | "photara.scalable-storage.v1"
                | "photara.sealed-roots.v1"
                | "photara.storage-accounting.v1"
                | "photara.whole-blob-storage.v1"
                | "photara.resource-retention-evidence.v1"
        ) {
            return Err(PackageError::UnsupportedFeature);
        }
        if previous.is_some_and(|p| p >= text) {
            return Err(PackageError::Record);
        }
        previous = Some(text);
        result.insert(text.to_owned());
    }
    Ok(result)
}
pub(super) fn nonnil(value: &Value) -> Result<PackageUuid, PackageError> {
    let id = PackageUuid::parse(value.as_str().ok_or(PackageError::Record)?)?;
    if id.as_uuid().is_nil() {
        Err(PackageError::Record)
    } else {
        Ok(id)
    }
}
pub(super) fn sha(value: &Value) -> Result<Sha256Hex, PackageError> {
    Sha256Hex::parse(value.as_str().ok_or(PackageError::Record)?)
}
pub(super) fn json_ref(value: &Value) -> Result<ObjectRef, PackageError> {
    let reference: ObjectRef =
        serde_json::from_value(value.clone()).map_err(|_| PackageError::Record)?;
    if reference.kind != ObjectKind::Json || reference.byte_length.get() == 0 {
        Err(PackageError::Record)
    } else {
        Ok(reference)
    }
}
fn physical(value: &Value) -> Result<PhysicalRef, PackageError> {
    let reference: PhysicalRef =
        serde_json::from_value(value.clone()).map_err(|_| PackageError::Record)?;
    if reference.allocation_id.as_uuid().is_nil() {
        Err(PackageError::Record)
    } else {
        Ok(reference)
    }
}
pub(super) fn schema(
    value: &Value,
    id: &str,
    version: u64,
    extra: &[&str],
    project: PackageUuid,
) -> Result<(), PackageError> {
    let mut names = vec!["schema", "project_id", "extensions"];
    names.extend(extra);
    fields(value, &names)?;
    if value["schema"] != json!({"id":id,"version":version})
        || value["project_id"] != project.to_string()
    {
        return Err(PackageError::UnsupportedVersion);
    }
    if value["extensions"]
        .as_object()
        .ok_or(PackageError::Record)?
        .keys()
        .any(|key| QualifiedName::parse(key.clone()).is_err())
    {
        return Err(PackageError::Record);
    }
    Ok(())
}
