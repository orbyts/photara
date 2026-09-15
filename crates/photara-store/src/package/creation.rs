//! Deterministic initial package bytes. No asset discovery or external archive reads.
use super::{ObjectKind, ObjectRef, PackageError, digest};
use photara_core::{
    GraphDocument, GraphId, canonical_json,
    creation::{InitialProject, project_name},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(unix)]
mod filesystem;
#[cfg(unix)]
pub use filesystem::*;

#[derive(Clone, Debug)]
pub struct InitialPackage {
    pub spec: InitialProject,
    pub files: BTreeMap<String, Vec<u8>>,
    pub commit_sha256: super::Sha256Hex,
    pub graph_sha256: super::Sha256Hex,
}

impl InitialPackage {
    /// Builds the full canonical package 1.1 closure without filesystem effects.
    /// # Errors
    /// Rejects invalid names, timestamps or canonical records.
    #[expect(
        clippy::too_many_lines,
        reason = "Initial package closure is assembled in dependency order"
    )]
    pub fn build(spec: InitialProject) -> Result<Self, PackageError> {
        if project_name(&spec.title).map_err(|_| PackageError::Record)? != spec.title {
            return Err(PackageError::Record);
        }
        super::records::timestamp(&spec.created_at)?;
        let p = spec.project_id;
        let t = &spec.created_at;
        let mut files = BTreeMap::new();
        let mut refs = BTreeSet::new();
        let record = |kind: &str, version: u32, mut value: Value| {
            value["schema"] = json!({"id":kind,"version":version});
            value["project_id"] = json!(p);
            value
        };
        let mut object =
            |kind: &str, version: u32, value: Value| -> Result<ObjectRef, PackageError> {
                let bytes = canonical_json(&record(kind, version, value))
                    .map_err(|_| PackageError::Record)?;
                let reference = ObjectRef {
                    kind: ObjectKind::Json,
                    sha256: digest(&bytes),
                    byte_length: super::DecimalU64::parse(&bytes.len().to_string())?,
                };
                files.insert(reference.path(), bytes);
                refs.insert(reference.clone());
                Ok(reference)
            };
        let parties = object(
            "photara.project.party-assignments",
            1,
            json!({"assignments":[]}),
        )?;
        let locations = object(
            "photara.project.location-assignments",
            1,
            json!({"assignments":[]}),
        )?;
        let assets = object("photara.project.asset-ledger", 1, json!({"assets":[]}))?;
        let resources = object(
            "photara.project.resource-ledger",
            1,
            json!({"managed_resources":[],"external_resources":[],"artifacts":[]}),
        )?;
        let context_value = |scope| json!({"scope":scope,"variables":[],"expressions":[],"captures":[],"metadata_selections":[],"node_contexts":[]});
        let context = object(
            "photara.context.authored",
            1,
            context_value(json!({"kind":"project","project_id":p})),
        )?;
        let graph_context = object(
            "photara.context.authored",
            1,
            context_value(json!({"kind":"graph","project_id":p,"graph_id":spec.graph_id})),
        )?;
        let graph = GraphDocument::new(GraphId::from_uuid(spec.graph_id.uuid()));
        let graph_sha256 = digest(&canonical_json(&graph).map_err(|_| PackageError::Record)?);
        let graph = object(
            "photara.project.saved-graph",
            2,
            json!({"graph_id":spec.graph_id,"name":"Graph 1","name_normalization_version":1,"metadata_revision":"1","created_at":t,"updated_at":t,"required_packages":[],"graph":graph,"context":graph_context,"node_contracts":[]}),
        )?;
        let authored = object(
            "photara.project.authored",
            2,
            json!({"owning_library_id":spec.library_id,"origin":{"library_id":null,"source_format":"photara.project-package.1.1"},"created_at":t,"updated_at":t,"title":spec.title,"description":"","lifecycle":"active","authored_revision":"1","party_assignments":parties,"location_assignments":locations,"asset_ledger":assets,"resource_ledger":resources,"context":context,"graphs":[{"graph_id":spec.graph_id,"document":graph}]}),
        )?;
        let history = object(
            "photara.project.history",
            2,
            json!({"runs":[],"operations":[],"evidence":[],"snapshots":[],"metadata_observations":[],"proposals":[],"apply_receipts":[],"artifacts":[]}),
        )?;
        let inventory = canonical_json(&record(
            "photara.package.inventory",
            1,
            json!({"objects":refs}),
        ))
        .map_err(|_| PackageError::Record)?;
        let inventory_ref = ObjectRef {
            kind: ObjectKind::Json,
            sha256: digest(&inventory),
            byte_length: super::DecimalU64::parse(&inventory.len().to_string())?,
        };
        files.insert(inventory_ref.path(), inventory);
        let features = [
            "photara.context.v1",
            "photara.history.v1",
            "photara.immutable-objects.v1",
            "photara.library-project.v1",
            "photara.node-contract.v2",
            "photara.resources.v2",
        ];
        let manifest = canonical_json(&json!({"format":"photara.project-package","format_version":{"major":1,"minor":1},"project_id":p,"created_at":t,"canonical_json":"photara.canonical-json.v1","required_features":features})).map_err(|_| PackageError::Record)?;
        let commit = canonical_json(&record("photara.package.commit",1,json!({"commit_id":spec.commit_id,"package_revision":"1","parent":null,"write_id":spec.operation_id,"created_at":t,"bootstrap_sha256":digest(&manifest),"minimum_reader":{"major":1,"minor":1},"required_features":features,"authored":authored,"history":history,"inventory":inventory_ref}))).map_err(|_| PackageError::Record)?;
        let commit_sha256 = digest(&commit);
        let head = canonical_json(&record(
            "photara.package.head",
            1,
            json!({"commit_id":spec.commit_id,"commit_sha256":commit_sha256}),
        ))
        .map_err(|_| PackageError::Record)?;
        files.insert("manifest.json".into(), manifest);
        files.insert(format!("commits/{}.json", spec.commit_id), commit);
        files.insert("HEAD.json".into(), head);
        Ok(Self {
            spec,
            files,
            commit_sha256,
            graph_sha256,
        })
    }
}
