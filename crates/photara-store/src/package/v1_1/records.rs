use super::*;

pub(super) fn expression(v: &Value) -> Result<cx::expression::Expression, PackageError> {
    let mut p = payload(v);
    let owner = p
        .as_object_mut()
        .unwrap()
        .remove("owner")
        .ok_or(PackageError::Record)?;
    if p.get("environment").and_then(|e| e.get("owner")) != Some(&owner) {
        return Err(PackageError::Integrity);
    }
    rename(&mut p, "source_utf8", "source")?;
    rename(&mut p, "bound_dependencies", "dependencies")?;
    checked(cx::expression::Expression::verify(decode(&p)?))
}
fn rename(v: &mut Value, from: &str, to: &str) -> Result<(), PackageError> {
    let m = v.as_object_mut().ok_or(PackageError::Record)?;
    if m.contains_key(to) {
        return Err(PackageError::Record);
    }
    let x = m.remove(from).ok_or(PackageError::Record)?;
    m.insert(to.into(), x);
    Ok(())
}
pub(super) fn snapshot(v: &Value) -> Result<cx::snapshot::ContextSnapshot, PackageError> {
    let mut spec = payload(v);
    spec["project_id"] = v["project_id"].clone();
    for k in [
        "content_digest",
        "completeness",
        "replayability",
        "captures",
        "expressions",
    ] {
        spec.as_object_mut().unwrap().remove(k);
    }
    let s = checked(cx::snapshot::ContextSnapshot::build(decode(&spec)?))?;
    let wire = serde_json::to_value(&s).map_err(|_| PackageError::Record)?;
    same(
        v,
        &wire,
        &["content_digest", "completeness", "replayability"],
    )?;
    Ok(s)
}
pub(super) fn variable(
    record_value: &Value,
    context: &Context,
) -> Result<cx::variable::VariableAggregate, PackageError> {
    let mut payload_value = payload(record_value);
    rename(&mut payload_value, "names", "claimed_names")?;
    let current = payload_value
        .as_object_mut()
        .unwrap()
        .remove("current_value")
        .ok_or(PackageError::Record)?;
    if current.is_null() {
        payload_value["current"] = Value::Null;
    } else {
        payload_value["current"] = field(&current, "binding")?.clone();
        same(&payload_value, &current, &["value_id", "origin"])?;
        if current
            .as_object()
            .ok_or(PackageError::Record)?
            .keys()
            .any(|k| !["binding", "value_id", "origin", "provenance"].contains(&k.as_str()))
        {
            return Err(PackageError::Record);
        }
    }
    payload_value.as_object_mut().unwrap().remove("provenance");
    for key in ["current", "default"] {
        let binding_value = field(&payload_value, key)?.clone();
        if binding_value.get("kind") == Some(&json!("expression")) {
            let reference_value = reference(&binding_value, "value")?;
            let expression_value = expression(target(
                context,
                &reference_value,
                "photara.context.expression",
            )?)?;
            payload_value[key]["value"] = serde_json::to_value(expression_value.record())
                .map_err(|_| PackageError::Record)?;
        }
    }
    decode(&payload_value)
}
fn strip(v: &Value, keys: &[&str]) -> Value {
    let mut p = v.clone();
    for k in keys {
        p.as_object_mut().unwrap().remove(*k);
    }
    p
}
fn local_base(v: &Value, diags: &mut Vec<PackageDiagnostic>) -> Result<(), PackageError> {
    let mut p = v.clone();
    p["schema"]["version"] = json!(1);
    super::super::records::validate_record(&p, decode(field(v, "project_id")?)?, diags)
}
fn portable_labels(v: &Value) -> Result<(), PackageError> {
    let _: cx::snapshot::Sensitivity = decode(field(v, "sensitivity")?)?;
    let portability: cx::snapshot::Portability = decode(field(v, "portability")?)?;
    if portability == cx::snapshot::Portability::HostOnly {
        return Err(PackageError::Record);
    }
    if portability == cx::snapshot::Portability::CaptureConsentRequired {
        hash(v, "consent_projection_digest")?;
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "Closed schema dispatch for D19 package records"
)]
pub(in super::super) fn validate_record(
    v: &Value,
    project: photara_core::ProjectId,
    diags: &mut Vec<PackageDiagnostic>,
) -> Result<(), PackageError> {
    if !known(v)? {
        return super::super::records::validate_record(v, project, diags);
    }
    if decode::<photara_core::ProjectId>(field(v, "project_id")?)? != project {
        return Err(PackageError::Integrity);
    }
    id(v, "project_id")?;
    if let Some(ext) = v.get("extensions") {
        for key in ext.as_object().ok_or(PackageError::Record)?.keys() {
            contract(ct::schema::QualifiedName::parse(key))?;
        }
    }
    // Portable envelopes have no active authority or host-state fields.
    for key in [
        "acl",
        "grants",
        "invitations",
        "host_binding_id",
        "host_path",
        "absolute_path",
        "credentials",
        "provider_connection",
        "device_snapshot",
        "sql",
        "containing_commit",
    ] {
        if v.get(key).is_some() {
            return Err(PackageError::Record);
        }
    }
    for k in [
        "created_at",
        "updated_at",
        "captured_at",
        "observed_at",
        "started_at",
        "queued_at",
        "verification_at",
    ] {
        if let Some(t) = v.get(k) {
            super::super::records::timestamp(t.as_str().ok_or(PackageError::Record)?)?;
        }
    }
    match kind(v)?.0 {
        "photara.project.authored" => {
            id(v, "owning_library_id")?;
            let origin = field(v, "origin")?;
            text(origin, "source_format")?;
            if origin.get("library_id").is_some_and(|x| !x.is_null()) {
                id(origin, "library_id")?;
            }
            let mut p = v.clone();
            p["schema"]["version"] = json!(1);
            p["originating_library_id"] = Value::Null;
            p["asset_inventory"] = field(v, "asset_ledger")?.clone();
            p["resource_inventory"] = field(v, "resource_ledger")?.clone();
            super::super::records::validate_record(&p, project, diags)?;
        }
        "photara.project.saved-graph" => {
            local_base(v, diags)?;
            let nodes = array(field(v, "graph")?, "nodes")?;
            let contracts = array(v, "node_contracts")?;
            ordered_ids(contracts, "node_id")?;
            if nodes.len() != contracts.len()
                || nodes
                    .iter()
                    .any(|n| !contracts.iter().any(|x| x.get("node_id") == n.get("id")))
            {
                return Err(PackageError::Integrity);
            }
            for n in contracts {
                hash(n, "contract_digest")?;
            }
        }
        "photara.project.library-snapshot" => {
            let mut p = v.clone();
            p["schema"]["version"] = json!(1);
            p["source"]["library_id"] = field(field(v, "source")?, "library_id")?.clone();
            let rev: ct::dto::RevisionCoordinate = decode(field(field(v, "source")?, "revision")?)?;
            p["source"]["revision"] = match rev {
                ct::dto::RevisionCoordinate::Local { revision } => {
                    json!({"authority":"local","value":revision.get().to_string()})
                }
                ct::dto::RevisionCoordinate::Server { revision } => {
                    json!({"authority":"service","value":String::from(revision)})
                }
                ct::dto::RevisionCoordinate::Package { .. } => return Err(PackageError::Record),
            };
            super::super::records::validate_record(&p, project, diags)?;
            let _: ct::schema::SchemaRef = decode(field(v, "projection_schema")?)?;
            let fields: Vec<ct::schema::LocalName> = decode(field(v, "fields")?)?;
            let declared: BTreeSet<_> = fields.iter().map(ct::schema::LocalName::as_str).collect();
            let present: BTreeSet<_> = field(v, "payload")?
                .as_object()
                .ok_or(PackageError::Record)?
                .keys()
                .map(String::as_str)
                .collect();
            if declared != present {
                return Err(PackageError::Integrity);
            }
            if fields.windows(2).any(|w| w[0] >= w[1]) {
                return Err(PackageError::Record);
            }
            portable_labels(v)?;
        }
        "photara.project.asset-ledger" => {
            let mut p = v.clone();
            p["schema"] = json!({"id":"photara.project.asset-inventory","version":1});
            super::super::records::validate_record(&p, project, diags)?;
            ordered_ids(array(v, "assets")?, "asset_id")?;
        }
        "photara.project.resource-ledger" => {
            for k in ["managed_resources", "external_resources", "artifacts"] {
                array(v, k)?;
            }
        }
        "photara.project.managed-resource" => {
            for k in ["resource_id", "version_id"] {
                id(v, k)?;
            }
            if text(v, "retention")? != "managed-project"
                || reference(v, "blob")?.kind != ObjectKind::Blob
            {
                return Err(PackageError::Record);
            }
            let _: ct::schema::MediaType = decode(field(field(v, "media")?, "media_type")?)?;
            if decimal(field(v, "media")?, "byte_length")?
                != reference(v, "blob")?.byte_length.get()
            {
                return Err(PackageError::Integrity);
            }
            field(v, "provenance")?;
        }
        "photara.project.external-resource" => {
            let _: ct::resource::ExternalResourceRevision =
                decode(&strip(v, &["schema", "extensions", "provenance"]))?;
            if !diags.contains(&PackageDiagnostic::ExternalResourceNotResolved) {
                diags.push(PackageDiagnostic::ExternalResourceNotResolved);
            }
        }
        "photara.project.representation-content" => {
            for k in ["asset_id", "representation_id", "content_revision_id"] {
                id(v, k)?;
            }
            let b = field(v, "binding")?;
            match text(b, "kind")? {
                "managed" => {
                    id(b, "resource_id")?;
                    id(b, "version_id")?;
                    reference(b, "version")?;
                }
                "external" => {
                    id(b, "external_ref_id")?;
                    id(b, "revision_id")?;
                    reference(b, "revision")?;
                }
                _ => return Err(PackageError::Record),
            }
            if b.as_object().unwrap().len() != 4 {
                return Err(PackageError::Record);
            }
            let _: ct::schema::MediaType = decode(field(field(v, "media")?, "media_type")?)?;
            decimal(field(v, "media")?, "byte_length")?;
            field(v, "fingerprint")?;
            array(v, "lineage")?;
        }
        "photara.value.asset-set-snapshot" => {
            id(v, "snapshot_id")?;
            uint(v, "member_count")?;
            hash(v, "content_digest")?;
            array(v, "pages")?;
            field(v, "provenance")?;
        }
        "photara.value.asset-set-page" => {
            id(v, "snapshot_id")?;
            uint(v, "start_ordinal")?;
            if array(v, "members")?.is_empty()
                || array(v, "members")?.len() > ct::asset_set::MAX_PAGE_MEMBERS
            {
                return Err(PackageError::Limit);
            }
        }
        "photara.context.authored" => {
            scope(field(v, "scope")?, field(v, "project_id")?)?;
            for k in [
                "variables",
                "expressions",
                "captures",
                "metadata_selections",
                "node_contexts",
            ] {
                array(v, k)?;
            }
            ordered_ids(array(v, "node_contexts")?, "node_id")?;
            if text(field(v, "scope")?, "kind")? != "graph"
                && !array(v, "node_contexts")?.is_empty()
            {
                return Err(PackageError::Record);
            }
        }
        "photara.context.expression" => {
            scope(field(v, "owner")?, field(v, "project_id")?)?;
            expression(v)?;
        }
        "photara.context.variable" => {
            scope(field(v, "scope")?, field(v, "project_id")?)?;
            id(v, "variable_id")?;
            array(v, "names")?;
            if text(v, "portability")? == "host-only" {
                return Err(PackageError::Record);
            }
        }
        "photara.context.snapshot" => {
            snapshot(v)?;
        }
        "photara.metadata.observation" => {
            super::super::records::timestamp(text(v, "captured_at")?)?;
            for k in [
                "observation_id",
                "asset_id",
                "representation_id",
                "content_revision_id",
            ] {
                id(v, k)?;
            }
            let _: ct::schema::SchemaRef = decode(field(v, "field_schema")?)?;
            let _: ct::schema::QualifiedName = decode(field(v, "field")?)?;
            let value: cx::value::TypedValue = decode(field(v, "value")?)?;
            checked(value.validate(false))?;
            field(v, "fingerprint")?;
            hash(field(v, "provenance")?, "digest")?;
            text(field(v, "provenance")?, "extractor")?;
            portable_labels(v)?;
        }
        "photara.metadata.patch" => {
            id(v, "patch_id")?;
            hash(v, "input_digest")?;
            let t = field(v, "target")?;
            if !["asset", "subset", "all-input", "group"].contains(&text(t, "kind")?) {
                return Err(PackageError::Record);
            }
            for op in array(v, "operations")? {
                let _: ct::schema::SchemaRef = decode(field(op, "field_schema")?)?;
                if !["add", "replace", "remove"].contains(&text(op, "kind")?) {
                    return Err(PackageError::Record);
                }
                if text(op, "kind")? != "remove" {
                    let x: cx::value::TypedValue = decode(field(op, "value")?)?;
                    checked(x.validate(false))?;
                }
                array(op, "preconditions")?;
            }
        }
        "photara.context.change-proposal" => {
            let _: cx::proposal::VariableChangeProposal =
                decode(&strip(v, &["schema", "extensions"]))?;
        }
        "photara.context.apply-receipt" => {
            let _: cx::proposal::ApplyReceipt = decode(&payload(v))?;
        }
        "photara.value.group-set" => {
            id(v, "group_snapshot_id")?;
            hash(v, "input_digest")?;
            ordered_ids(array(v, "groups")?, "group_id")?;
        }
        "photara.history.artifact" => {
            id(v, "artifact_id")?;
            id(v, "operation_id")?;
            let _: ct::resource::StorageClass = decode(field(v, "storage_class")?)?;
            text(v, "retention")?;
            text(v, "durability")?;
            text(v, "availability")?;
        }
        "photara.project.history" => {
            local_base(v, diags)?;
            for k in [
                "snapshots",
                "metadata_observations",
                "proposals",
                "apply_receipts",
                "artifacts",
            ] {
                array(v, k)?;
            }
        }
        "photara.history.run-start" => {
            local_base(v, diags)?;
            id(v, "owning_library_id")?;
            array(v, "input_snapshots")?;
            let auth = field(v, "authorization_observation")?;
            let _: ct::access::ActionMask = decode(field(auth, "actions")?)?;
            text(auth, "mode")?;
            let _: cx::snapshot::Replayability = decode(field(v, "replayability")?)?;
            if v.get("device_dependency_digest")
                .is_some_and(|x| !x.is_null())
            {
                hash(v, "device_dependency_digest")?;
            }
        }
        "photara.history.attempt-start" => {
            local_base(v, diags)?;
            for k in ["node_context_digest", "execution_contract_digest"] {
                hash(v, k)?;
            }
            array(v, "input_snapshot_refs")?;
            array(v, "operation_refs")?;
            let _: cx::snapshot::Replayability = decode(field(v, "replayability")?)?;
        }
        "photara.history.effect-intent" => {
            local_base(v, diags)?;
            let _: cx::cache::DefinitionPin = decode(field(v, "definition")?)?;
            let target: ct::resource::ResourceDescriptor = decode(field(v, "target")?)?;
            if matches!(target, ct::resource::ResourceDescriptor::HostPlace { .. }) {
                return Err(PackageError::Record);
            }
            for k in ["input_digest", "context_digest"] {
                hash(v, k)?;
            }
            for k in ["operation_kind", "retention"] {
                text(v, k)?;
            }
            let request: cx::value::TypedValue = decode(field(v, "request")?)?;
            checked(request.validate(false))?;
            field(v, "expected_target")?;
        }
        "photara.history.receipt" => {
            local_base(v, diags)?;
            hash(v, "request_digest")?;
            if !["succeeded", "rejected", "failed", "unknown"]
                .contains(&text(field(v, "observation")?, "kind")?)
            {
                return Err(PackageError::Record);
            }
            let ids: Vec<ct::ids::ReceiptId> = decode(field(v, "prior_receipt_ids")?)?;
            if ids.windows(2).any(|w| w[0] >= w[1]) {
                return Err(PackageError::Record);
            }
        }
        "photara.node.manifest" => {
            let _: photara_node_sdk::v2::NodePackageManifestV2 = decode(field(v, "manifest")?)?;
        }
        _ => return Err(PackageError::UnsupportedVersion),
    }
    let mut refs = Vec::new();
    references(v, &mut refs)?;
    Ok(())
}
