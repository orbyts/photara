use super::*;
use photara_core::{NodeGraphDocument, NodeGraphMetadata, PackageRequirement};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, PackageError> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or(PackageError::Record)
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, PackageError> {
    v.get(key)
        .and_then(Value::as_array)
        .ok_or(PackageError::Record)
}
fn field<'a>(v: &'a Value, key: &str) -> Result<&'a Value, PackageError> {
    v.get(key).ok_or(PackageError::Record)
}
pub(super) fn uuid_field(v: &Value, key: &str) -> Result<PackageUuid, PackageError> {
    PackageUuid::parse(text(v, key)?)
}
fn decimal(v: &Value, key: &str) -> Result<u64, PackageError> {
    Ok(DecimalU64::parse(text(v, key)?)?.get())
}
fn positive(v: &Value, key: &str) -> Result<(), PackageError> {
    if decimal(v, key)? == 0 {
        Err(PackageError::Record)
    } else {
        Ok(())
    }
}
fn hash(v: &Value, key: &str) -> Result<Sha256Hex, PackageError> {
    Sha256Hex::parse(text(v, key)?)
}
fn object(v: &Value, key: &str) -> Result<(), PackageError> {
    if !field(v, key)?.is_object() {
        return Err(PackageError::Record);
    }
    Ok(())
}
fn ref_field(v: &Value, key: &str) -> Result<ObjectRef, PackageError> {
    decode(field(v, key)?)
}
fn strings(v: &Value, key: &str) -> Result<Vec<String>, PackageError> {
    decode(field(v, key)?)
}
fn set(v: &Value, key: &str, nonempty: bool) -> Result<(), PackageError> {
    let values = strings(v, key)?;
    if (nonempty && values.is_empty())
        || values.windows(2).any(|p| p[0] >= p[1])
        || values.iter().any(String::is_empty)
    {
        return Err(PackageError::Record);
    }
    Ok(())
}
fn ids(v: &Value, keys: &[&str]) -> Result<(), PackageError> {
    for key in keys {
        uuid_field(v, key)?;
    }
    Ok(())
}
fn refs(v: &Value, key: &str) -> Result<(), PackageError> {
    for r in array(v, key)? {
        let _: ObjectRef = decode(r)?;
    }
    Ok(())
}
fn owned_schema(v: &Value) -> Result<RecordSchema, PackageError> {
    decode(field(v, "schema")?)
}
pub(super) fn require_kind(v: &Value, kind: &str) -> Result<(), PackageError> {
    schema(&owned_schema(v)?, kind)
}
fn one_of(v: &Value, key: &str, allowed: &[&str]) -> Result<(), PackageError> {
    if !allowed.contains(&text(v, key)?) {
        return Err(PackageError::Record);
    }
    Ok(())
}
fn named(v: &Value, key: &str, max: usize) -> Result<(), PackageError> {
    let s = text(v, key)?;
    if s.trim().is_empty() || s.len() > max {
        return Err(PackageError::Record);
    }
    Ok(())
}

pub(super) fn timestamp(s: &str) -> Result<(), PackageError> {
    if s.len() != 24
        || !s.is_ascii()
        || &s[10..11] != "T"
        || &s[13..14] != ":"
        || &s[16..17] != ":"
        || &s[19..20] != "."
        || &s[23..] != "Z"
    {
        return Err(PackageError::Record);
    }
    date(&s[..10])?;
    let number = |r: std::ops::Range<usize>| {
        let digits = &s[r];
        if !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(PackageError::Record);
        }
        digits.parse::<u32>().map_err(|_| PackageError::Record)
    };
    if number(11..13)? > 23 || number(14..16)? > 59 || number(17..19)? > 59 || number(20..23)? > 999
    {
        return Err(PackageError::Record);
    }
    Ok(())
}
fn date(s: &str) -> Result<(), PackageError> {
    if s.len() != 10 || !s.is_ascii() || &s[4..5] != "-" || &s[7..8] != "-" {
        return Err(PackageError::Record);
    }
    if !s[..4]
        .bytes()
        .chain(s[5..7].bytes())
        .chain(s[8..].bytes())
        .all(|b| b.is_ascii_digit())
    {
        return Err(PackageError::Record);
    }
    let year = s[..4].parse::<u32>().map_err(|_| PackageError::Record)?;
    let month = s[5..7].parse::<usize>().map_err(|_| PackageError::Record)?;
    let day = s[8..].parse::<u32>().map_err(|_| PackageError::Record)?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month) || day == 0 || day > days[month - 1] {
        return Err(PackageError::Record);
    }
    Ok(())
}

/// Reference-bearing fields are schema-selected. Opaque node configuration,
/// provider evidence values and optional extensions are never reinterpreted as
/// filesystem references merely because they resemble an `ObjectRef`.
#[expect(
    clippy::items_after_statements,
    reason = "The local recursive helper follows the schema-selected field list"
)]
pub(super) fn references(v: &Value, out: &mut Vec<ObjectRef>) -> Result<(), PackageError> {
    let s = owned_schema(v)?;
    let keys: &[&str] = match s.id.as_str() {
        "photara.project.authored" => &[
            "party_assignments",
            "location_assignments",
            "asset_inventory",
            "resource_inventory",
            "graphs",
        ],
        "photara.package.inventory" => &["objects"],
        "photara.project.party-assignments" | "photara.project.location-assignments" => {
            &["assignments"]
        }
        "photara.project.party-assignment" => &["party_snapshot"],
        "photara.project.location-assignment" => &["location_snapshot", "location_kind_snapshot"],
        "photara.project.asset-inventory" => &["assets"],
        "photara.project.resource-inventory" => &["resources"],
        "photara.project.representation-content" => &["binding", "lineage"],
        "photara.project.saved-graph" => &["required_packages"],
        "photara.project.history" => &["runs", "operations", "evidence"],
        "photara.history.run" => &[
            "start",
            "attempts",
            "outcome",
            "operations",
            "receipts",
            "evidence",
        ],
        "photara.history.run-start" => &["source_authored", "dependencies"],
        "photara.history.attempt-start" => &["representation_revisions"],
        "photara.history.attempt-outcome" | "photara.history.run-outcome" => {
            &["evidence", "artifacts"]
        }
        "photara.history.receipt" => &["evidence"],
        "photara.project.library-snapshot"
        | "photara.history.effect-intent"
        | "photara.history.evidence" => &[],
        _ => return Err(PackageError::UnsupportedVersion),
    };
    fn walk(v: &Value, out: &mut Vec<ObjectRef>) -> Result<(), PackageError> {
        if v.is_object() && matches!(v.get("kind").and_then(Value::as_str), Some("json" | "blob")) {
            out.push(decode(v)?);
        } else if let Some(a) = v.as_array() {
            for x in a {
                walk(x, out)?;
            }
        } else if let Some(m) = v.as_object() {
            for x in m.values() {
                walk(x, out)?;
            }
        }
        Ok(())
    }
    for key in keys {
        if let Some(value) = v.get(key) {
            walk(value, out)?;
        }
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "One auditable dispatch enumerates every supported v1 envelope"
)]
pub(super) fn validate_record(
    v: &Value,
    project: photara_core::ProjectId,
    diagnostics: &mut Vec<PackageDiagnostic>,
) -> Result<(), PackageError> {
    uuid_field(v, "project_id")?;
    let envelope: OwnedRecord = decode(v)?;
    if envelope.project_id != project {
        return Err(PackageError::Integrity);
    }
    if envelope.schema.version != 1 {
        return Err(PackageError::UnsupportedVersion);
    }
    for key in [
        "created_at",
        "updated_at",
        "captured_at",
        "queued_at",
        "started_at",
        "ended_at",
        "verification_at",
    ] {
        if let Some(value) = v.get(key) {
            timestamp(value.as_str().ok_or(PackageError::Record)?)?;
        }
    }
    let required_times: &[&str] = match envelope.schema.id.as_str() {
        "photara.project.party-assignment" | "photara.project.location-assignment" => {
            &["created_at", "updated_at"]
        }
        "photara.project.library-snapshot"
        | "photara.history.effect-intent"
        | "photara.history.evidence" => &["captured_at"],
        "photara.history.run-start" => &["queued_at", "started_at"],
        "photara.history.attempt-start" => &["started_at"],
        "photara.history.receipt" => &["verification_at"],
        _ => &[],
    };
    for key in required_times {
        timestamp(text(v, key)?)?;
    }
    match envelope.schema.id.as_str() {
        "photara.package.inventory" => {
            let _: Inventory = decode(v)?;
        }
        "photara.project.authored" => {
            let a: AuthoredProject = decode(v)?;
            if a.authored_revision.get() == 0
                || a.title.trim().is_empty()
                || a.title.len() > 512
                || a.description.len() > 8192
                || !matches!(a.lifecycle.as_str(), "active" | "archived")
            {
                return Err(PackageError::Record);
            }
        }
        "photara.project.saved-graph" => validate_graph(v, diagnostics)?,
        "photara.project.party-assignments" | "photara.project.location-assignments" => {
            refs(v, "assignments")?;
        }
        "photara.project.party-assignment" => {
            ids(v, &["assignment_id"])?;
            positive(v, "revision")?;
            ref_field(v, "party_snapshot")?;
            set(v, "roles", true)?;
            text(v, "notes")?;
        }
        "photara.project.location-assignment" => {
            uuid_field(v, "assignment_id")?;
            positive(v, "revision")?;
            ref_field(v, "location_snapshot")?;
            ref_field(v, "location_kind_snapshot")?;
            text(v, "notes")?;
            let schedule = field(v, "schedule")?;
            if !schedule.is_null() {
                match text(schedule, "kind")? {
                    "calendar-date" => date(text(schedule, "date")?)?,
                    "utc-interval" => {
                        let start = text(schedule, "start")?;
                        let end = text(schedule, "end")?;
                        timestamp(start)?;
                        timestamp(end)?;
                        if end <= start {
                            return Err(PackageError::Record);
                        }
                        named(schedule, "display_zone", 256)?;
                    }
                    _ => return Err(PackageError::UnsupportedVersion),
                }
            }
            let mut seen = BTreeSet::new();
            for p in array(v, "participants")? {
                if !seen.insert(uuid_field(p, "party_assignment_id")?) {
                    return Err(PackageError::Record);
                }
                set(p, "roles", true)?;
            }
        }
        "photara.project.library-snapshot" => {
            uuid_field(v, "snapshot_id")?;
            named(v, "display_name", 512)?;
            let source = field(v, "source")?;
            ids(source, &["library_id", "record_id"])?;
            one_of(
                source,
                "kind",
                &["person", "organization", "location", "location_kind"],
            )?;
            let revision = field(source, "revision")?;
            one_of(revision, "authority", &["local", "service"])?;
            let token = text(revision, "value")?;
            let decimal_token = if text(revision, "authority")? == "service" {
                token.strip_prefix("sr1:").ok_or(PackageError::Record)?
            } else {
                token
            };
            let revision_number = DecimalU64::parse(decimal_token)?.get();
            if revision_number == 0 || revision_number > 9_223_372_036_854_775_807 {
                return Err(PackageError::Record);
            }
            let payload = field(v, "payload")?;
            object(v, "payload")?;
            match text(source, "kind")? {
                "location" => {
                    uuid_field(payload, "location_kind_id")?;
                    named(payload, "display_name", 512)?;
                }
                "location_kind" => {
                    named(payload, "canonical_key", 512)?;
                    named(payload, "canonical_display", 512)?;
                    strings(payload, "aliases")?;
                    if field(payload, "normalization_version")?.as_u64() != Some(1) {
                        return Err(PackageError::UnsupportedVersion);
                    }
                }
                _ => named(payload, "display_name", 512)?,
            }
        }
        "photara.project.asset-inventory" => {
            let mut seen = BTreeSet::new();
            let mut representations = BTreeSet::new();
            for asset in array(v, "assets")? {
                if !seen.insert(uuid_field(asset, "asset_id")?) {
                    return Err(PackageError::Record);
                }
                named(asset, "display_name", 512)?;
                positive(asset, "revision")?;
                timestamp(text(asset, "created_at")?)?;
                timestamp(text(asset, "updated_at")?)?;
                for r in array(asset, "representations")? {
                    if !representations.insert(uuid_field(r, "representation_id")?) {
                        return Err(PackageError::Record);
                    }
                    set(r, "roles", true)?;
                    set(r, "capabilities", false)?;
                    ref_field(r, "current_content")?;
                }
            }
        }
        "photara.project.resource-inventory" => {
            let mut seen = BTreeSet::new();
            for r in array(v, "resources")? {
                if !seen.insert(uuid_field(r, "resource_id")?) {
                    return Err(PackageError::Record);
                }
                positive(r, "version")?;
                if ref_field(r, "blob")?.kind != ObjectKind::Blob {
                    return Err(PackageError::Record);
                }
                text(r, "original_filename")?;
            }
        }
        "photara.project.representation-content" => validate_content(v, diagnostics)?,
        "photara.project.history" => {
            let mut previous = None;
            for r in array(v, "runs")? {
                let id = uuid_field(r, "run_id")?;
                if previous.is_some_and(|p| p >= id) {
                    return Err(PackageError::Record);
                }
                previous = Some(id);
                ref_field(r, "document")?;
            }
            refs(v, "operations")?;
            refs(v, "evidence")?;
        }
        "photara.history.run-start" => {
            ids(v, &["run_id", "request_id", "source_graph_id"])?;
            decimal(v, "source_graph_revision")?;
            hash(v, "source_graph_digest")?;
            ref_field(v, "source_authored")?;
            refs(v, "dependencies")?;
            for target in array(v, "targets")? {
                PackageUuid::parse(target.as_str().ok_or(PackageError::Record)?)?;
            }
            for implementation in array(v, "implementations")? {
                validate_pin(implementation)?;
            }
        }
        "photara.history.attempt-start" => {
            ids(v, &["run_id", "attempt_id", "node_id"])?;
            let ordinal = field(v, "ordinal")?.as_u64().ok_or(PackageError::Record)?;
            if ordinal == 0 || ordinal > u64::from(u32::MAX) {
                return Err(PackageError::Record);
            }
            for key in ["configuration_digest", "input_digest", "environment_digest"] {
                hash(v, key)?;
            }
            let implementation = field(v, "implementation")?;
            hash(implementation, "fingerprint")?;
            validate_pin(implementation)?;
            refs(v, "representation_revisions")?;
        }
        "photara.history.effect-intent" => {
            ids(v, &["run_id", "attempt_id", "operation_id"])?;
            object(v, "target")?;
            hash(v, "request_digest")?;
            named(v, "idempotency_key", 256)?;
        }
        "photara.history.evidence" => {
            ids(v, &["evidence_id", "operation_id"])?;
            object(v, "provenance")?;
            let content = field(v, "content")?;
            let _: RecordSchema = decode(field(content, "schema")?)?;
            field(content, "value")?;
        }
        "photara.history.receipt" => {
            ids(v, &["receipt_id", "operation_id", "observing_attempt_id"])?;
            named(v, "provider", 256)?;
            refs(v, "evidence")?;
            object(v, "observation")?;
        }
        "photara.history.attempt-outcome" | "photara.history.run-outcome" => {
            uuid_field(v, "run_id")?;
            one_of(
                v,
                "status",
                &["succeeded", "failed", "cancelled", "interrupted"],
            )?;
            timestamp(text(v, "ended_at")?)?;
            array(v, "diagnostics")?;
            refs(v, "evidence")?;
            if envelope.schema.id == "photara.history.attempt-outcome" {
                uuid_field(v, "attempt_id")?;
                for operation in array(v, "operations")? {
                    uuid_field(operation, "operation_id")?;
                    one_of(
                        operation,
                        "knowledge",
                        &["unknown", "confirmed", "not-applied"],
                    )?;
                }
            }
        }
        "photara.history.run" => {
            uuid_field(v, "run_id")?;
            ref_field(v, "start")?;
            ref_field(v, "outcome")?;
            refs(v, "operations")?;
            refs(v, "receipts")?;
            refs(v, "evidence")?;
            let mut seen = BTreeSet::new();
            for a in array(v, "attempts")? {
                if !seen.insert(uuid_field(a, "attempt_id")?) {
                    return Err(PackageError::Record);
                }
                ref_field(a, "start")?;
                ref_field(a, "outcome")?;
            }
        }
        _ => return Err(PackageError::UnsupportedVersion),
    }
    Ok(())
}

fn validate_pin(value: &Value) -> Result<(), PackageError> {
    let pin: photara_core::NodeDefinitionRef = decode(value)?;
    photara_core::NodePackageId::parse(pin.package_id.as_str())
        .map_err(|_| PackageError::Record)?;
    photara_core::NodeDefinitionId::parse(pin.definition_id.as_str())
        .map_err(|_| PackageError::Record)?;
    if pin.definition_version.get() == 0 {
        return Err(PackageError::Record);
    }
    Ok(())
}

fn validate_schema_value(value: &Value) -> Result<(), PackageError> {
    let schema = field(value, "schema")?;
    photara_core::SchemaId::parse(text(schema, "id")?).map_err(|_| PackageError::Record)?;
    let version = field(schema, "version")?
        .as_u64()
        .ok_or(PackageError::Record)?;
    if version == 0 || version > u64::from(u32::MAX) {
        return Err(PackageError::Record);
    }
    field(value, "value")?;
    Ok(())
}

fn validate_graph(v: &Value, diagnostics: &mut Vec<PackageDiagnostic>) -> Result<(), PackageError> {
    let saved: SavedGraph = decode(v)?;
    if saved.metadata_revision.get() == 0 || saved.name.trim().is_empty() || saved.name.len() > 512
    {
        return Err(PackageError::Record);
    }
    if saved.name_normalization_version != 1 {
        return Err(PackageError::UnsupportedVersion);
    }
    if !saved.name.is_ascii()
        && !diagnostics.contains(&PackageDiagnostic::GraphNameNormalizationUnsupported)
    {
        diagnostics.push(PackageDiagnostic::GraphNameNormalizationUnsupported);
    }
    let graph = field(v, "graph")?;
    if uuid_field(graph, "id")? != saved.graph_id {
        return Err(PackageError::Integrity);
    }
    for node in array(graph, "nodes")? {
        uuid_field(node, "id")?;
        validate_pin(field(node, "definition")?)?;
        validate_schema_value(field(node, "configuration")?)?;
        if let Some(state) = node.get("authored_state") {
            validate_schema_value(state)?;
        }
    }
    for c in array(graph, "connections")? {
        uuid_field(c, "id")?;
        uuid_field(field(c, "input")?, "node_id")?;
        uuid_field(field(c, "output")?, "node_id")?;
        for endpoint in ["input", "output"] {
            photara_core::PortId::parse(text(field(c, endpoint)?, "port_id")?)
                .map_err(|_| PackageError::Record)?;
        }
    }
    let requirements: Vec<_> = saved
        .required_packages
        .iter()
        .map(|p| PackageRequirement {
            package_id: p.package_id.clone(),
            package_version: p.package_version.clone(),
        })
        .collect();
    let actual: BTreeSet<_> = saved
        .graph
        .nodes
        .iter()
        .map(|n| PackageRequirement {
            package_id: n.definition.package_id.clone(),
            package_version: n.definition.package_version.clone(),
        })
        .collect();
    if requirements.iter().cloned().collect::<BTreeSet<_>>() != actual {
        return Err(PackageError::Record);
    }
    NodeGraphDocument {
        schema_version: photara_core::SchemaVersion::first(),
        metadata: NodeGraphMetadata {
            name: saved.name.clone(),
            description: None,
        },
        required_packages: requirements,
        graph: saved.graph,
        extensions: BTreeMap::new(),
    }
    .validate()
    .map_err(|_| PackageError::Record)?;
    if saved.required_packages.iter().any(|p| p.manifest.is_none())
        && !diagnostics.contains(&PackageDiagnostic::NodeManifestUnavailable)
    {
        diagnostics.push(PackageDiagnostic::NodeManifestUnavailable);
    }
    Ok(())
}

fn validate_content(
    v: &Value,
    diagnostics: &mut Vec<PackageDiagnostic>,
) -> Result<(), PackageError> {
    ids(v, &["asset_id", "representation_id", "content_revision_id"])?;
    let media = field(v, "media")?;
    named(media, "media_type", 256)?;
    let size = decimal(media, "byte_length")?;
    for dimension in ["width", "height"] {
        if let Some(value) = media.get(dimension).filter(|value| !value.is_null()) {
            let number = value.as_u64().ok_or(PackageError::Record)?;
            if number == 0 || number > u64::from(u32::MAX) {
                return Err(PackageError::Record);
            }
        }
    }
    array(v, "lineage")?;
    let fingerprint = field(v, "fingerprint")?;
    one_of(
        fingerprint,
        "kind",
        &["content-digest", "provider-revision", "file-observation"],
    )?;
    one_of(
        fingerprint,
        "evidence_strength",
        &["verified-bytes", "observation-only"],
    )?;
    if text(fingerprint, "kind")? == "content-digest" {
        if text(fingerprint, "algorithm")? != "sha256" {
            return Err(PackageError::UnsupportedVersion);
        }
        hash(fingerprint, "value")?;
    }
    let binding = field(v, "binding")?;
    match text(binding, "kind")? {
        "managed" => {
            uuid_field(binding, "resource_id")?;
            positive(binding, "resource_version")?;
            let blob = ref_field(binding, "blob")?;
            if blob.kind != ObjectKind::Blob
                || blob.byte_length.get() != size
                || text(binding, "path")? != blob.path()
            {
                return Err(PackageError::Integrity);
            }
            if text(fingerprint, "kind")? == "content-digest"
                && (text(fingerprint, "algorithm")? != "sha256"
                    || text(fingerprint, "value")? != blob.sha256.as_str())
            {
                return Err(PackageError::Integrity);
            }
        }
        "external" => {
            uuid_field(binding, "storage_binding_id")?;
            if !field(binding, "storage_root_id")?.is_null() {
                uuid_field(binding, "storage_root_id")?;
            }
            validate_resource_path(text(binding, "relative_path")?)?;
            if !diagnostics.contains(&PackageDiagnostic::ExternalResourceNotResolved) {
                diagnostics.push(PackageDiagnostic::ExternalResourceNotResolved);
            }
        }
        _ => return Err(PackageError::UnsupportedVersion),
    }
    if text(fingerprint, "kind")? == "file-observation"
        && text(fingerprint, "evidence_strength")? == "verified-bytes"
    {
        return Err(PackageError::Record);
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "Checks follow the authored aggregate's graph, party, location and asset boundaries"
)]
pub(super) fn validate_authored(
    a: &AuthoredProject,
    c: &impl ObjectLookup,
) -> Result<(), PackageError> {
    schema(&a.schema, "photara.project.authored")?;
    if a.graphs.len() > 1000 || a.graphs.windows(2).any(|p| p[0].graph_id >= p[1].graph_id) {
        return Err(PackageError::Record);
    }
    let mut names = BTreeSet::new();
    for graph in &a.graphs {
        let v = &c.object(&graph.document)?.value;
        require_kind(v, "photara.project.saved-graph")?;
        if uuid_field(v, "graph_id")? != graph.graph_id {
            return Err(PackageError::Integrity);
        }
        let name = text(v, "name")?;
        if name.is_ascii()
            && !names.insert(
                name.split_ascii_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_ascii_lowercase(),
            )
        {
            return Err(PackageError::Record);
        }
    }
    let parties = &c.object(&a.party_assignments)?.value;
    require_kind(parties, "photara.project.party-assignments")?;
    let mut party_ids = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for r in array(parties, "assignments")? {
        let p = &c.object(&decode(r)?)?.value;
        require_kind(p, "photara.project.party-assignment")?;
        if !party_ids.insert(uuid_field(p, "assignment_id")?) {
            return Err(PackageError::Record);
        }
        let snap = &c.object(&ref_field(p, "party_snapshot")?)?.value;
        require_kind(snap, "photara.project.library-snapshot")?;
        let source = field(snap, "source")?;
        one_of(source, "kind", &["person", "organization"])?;
        if !sources.insert((
            uuid_field(source, "library_id")?,
            text(source, "kind")?.to_owned(),
            uuid_field(source, "record_id")?,
        )) {
            return Err(PackageError::Record);
        }
    }
    let locations = &c.object(&a.location_assignments)?.value;
    require_kind(locations, "photara.project.location-assignments")?;
    let mut location_ids = BTreeSet::new();
    for r in array(locations, "assignments")? {
        let assignment = &c.object(&decode(r)?)?.value;
        require_kind(assignment, "photara.project.location-assignment")?;
        if !location_ids.insert(uuid_field(assignment, "assignment_id")?) {
            return Err(PackageError::Record);
        }
        let location = &c
            .object(&ref_field(assignment, "location_snapshot")?)?
            .value;
        let kind = &c
            .object(&ref_field(assignment, "location_kind_snapshot")?)?
            .value;
        require_kind(location, "photara.project.library-snapshot")?;
        require_kind(kind, "photara.project.library-snapshot")?;
        if text(field(location, "source")?, "kind")? != "location"
            || text(field(kind, "source")?, "kind")? != "location_kind"
            || uuid_field(field(location, "payload")?, "location_kind_id")?
                != uuid_field(field(kind, "source")?, "record_id")?
            || uuid_field(field(location, "source")?, "library_id")?
                != uuid_field(field(kind, "source")?, "library_id")?
        {
            return Err(PackageError::Integrity);
        }
        for participant in array(assignment, "participants")? {
            if !party_ids.contains(&uuid_field(participant, "party_assignment_id")?) {
                return Err(PackageError::Integrity);
            }
        }
    }
    let assets = &c.object(&a.asset_inventory)?.value;
    require_kind(assets, "photara.project.asset-inventory")?;
    let resources = &c.object(&a.resource_inventory)?.value;
    require_kind(resources, "photara.project.resource-inventory")?;
    for asset in array(assets, "assets")? {
        for representation in array(asset, "representations")? {
            let content = &c
                .object(&ref_field(representation, "current_content")?)?
                .value;
            require_kind(content, "photara.project.representation-content")?;
            if uuid_field(asset, "asset_id")? != uuid_field(content, "asset_id")?
                || uuid_field(representation, "representation_id")?
                    != uuid_field(content, "representation_id")?
            {
                return Err(PackageError::Integrity);
            }
            let binding = field(content, "binding")?;
            if text(binding, "kind")? == "managed" {
                let resource = array(resources, "resources")?
                    .iter()
                    .find(|r| r.get("resource_id") == binding.get("resource_id"))
                    .ok_or(PackageError::Integrity)?;
                if decimal(resource, "version")? != decimal(binding, "resource_version")?
                    || ref_field(resource, "blob")? != ref_field(binding, "blob")?
                {
                    return Err(PackageError::Integrity);
                }
            }
        }
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "History uniqueness and cross-record checks are reviewed together"
)]
#[expect(
    clippy::collapsible_match,
    reason = "Side-effecting index insertion stays in explicit schema arms, not match guards"
)]
pub(super) fn validate_links(c: &impl ObjectLookup) -> Result<(), PackageError> {
    let mut starts = BTreeMap::new();
    let mut attempts = BTreeMap::new();
    let mut operations = BTreeMap::new();
    let mut terminals = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    for object in c.objects().values() {
        let v = &object.value;
        match owned_schema(v)?.id.as_str() {
            "photara.history.run-start" => {
                if starts.insert(uuid_field(v, "run_id")?, v).is_some() {
                    return Err(PackageError::Integrity);
                }
            }
            "photara.history.attempt-start" => {
                if attempts.insert(uuid_field(v, "attempt_id")?, v).is_some()
                    || !ordinals.insert((
                        uuid_field(v, "run_id")?,
                        uuid_field(v, "node_id")?,
                        field(v, "ordinal")?.as_u64(),
                    ))
                {
                    return Err(PackageError::Integrity);
                }
            }
            "photara.history.effect-intent" => {
                if operations
                    .insert(uuid_field(v, "operation_id")?, v)
                    .is_some()
                {
                    return Err(PackageError::Integrity);
                }
            }
            "photara.history.attempt-outcome" => {
                if !terminals.insert(("attempt", uuid_field(v, "attempt_id")?)) {
                    return Err(PackageError::Integrity);
                }
            }
            "photara.history.run-outcome" => {
                if !terminals.insert(("run", uuid_field(v, "run_id")?)) {
                    return Err(PackageError::Integrity);
                }
            }
            _ => {}
        }
    }
    for object in c.objects().values() {
        let v = &object.value;
        let s = owned_schema(v)?;
        match s.id.as_str() {
            "photara.history.run-start" => {
                let authored: AuthoredProject =
                    decode(&c.object(&ref_field(v, "source_authored")?)?.value)?;
                validate_authored(&authored, c)?;
                let graph_ref = authored
                    .graphs
                    .iter()
                    .find(|r| Some(r.graph_id) == uuid_field(v, "source_graph_id").ok())
                    .ok_or(PackageError::Integrity)?;
                let graph = &c.object(&graph_ref.document)?.value;
                let raw = field(graph, "graph")?;
                if field(raw, "revision")?.as_u64() != Some(decimal(v, "source_graph_revision")?)
                    || digest(&photara_core::canonical_json(raw).map_err(|_| PackageError::Record)?)
                        != hash(v, "source_graph_digest")?
                {
                    return Err(PackageError::Integrity);
                }
                let nodes = array(raw, "nodes")?;
                for target in array(v, "targets")? {
                    if !nodes.iter().any(|n| n.get("id") == Some(target)) {
                        return Err(PackageError::Integrity);
                    }
                }
            }
            "photara.history.attempt-start" => {
                let start = starts
                    .get(&uuid_field(v, "run_id")?)
                    .ok_or(PackageError::Integrity)?;
                let authored: AuthoredProject =
                    decode(&c.object(&ref_field(start, "source_authored")?)?.value)?;
                let g = authored
                    .graphs
                    .iter()
                    .find(|g| Some(g.graph_id) == uuid_field(start, "source_graph_id").ok())
                    .ok_or(PackageError::Integrity)?;
                let graph = field(&c.object(&g.document)?.value, "graph")?;
                let node = array(graph, "nodes")?
                    .iter()
                    .find(|n| n.get("id") == v.get("node_id"))
                    .ok_or(PackageError::Integrity)?;
                if digest(
                    &photara_core::canonical_json(field(node, "configuration")?)
                        .map_err(|_| PackageError::Record)?,
                ) != hash(v, "configuration_digest")?
                {
                    return Err(PackageError::Integrity);
                }
                let pin: photara_core::NodeDefinitionRef = decode(field(v, "implementation")?)?;
                let expected: photara_core::NodeDefinitionRef = decode(field(node, "definition")?)?;
                if pin != expected {
                    return Err(PackageError::Integrity);
                }
                for r in array(v, "representation_revisions")? {
                    require_kind(
                        &c.object(&decode(r)?)?.value,
                        "photara.project.representation-content",
                    )?;
                }
            }
            "photara.history.effect-intent" | "photara.history.attempt-outcome" => {
                let attempt = attempts
                    .get(&uuid_field(v, "attempt_id")?)
                    .ok_or(PackageError::Integrity)?;
                if attempt.get("run_id") != v.get("run_id") {
                    return Err(PackageError::Integrity);
                }
                if s.id == "photara.history.attempt-outcome" {
                    for op in array(v, "operations")? {
                        let intent = operations
                            .get(&uuid_field(op, "operation_id")?)
                            .ok_or(PackageError::Integrity)?;
                        if intent.get("attempt_id") != v.get("attempt_id") {
                            return Err(PackageError::Integrity);
                        }
                    }
                }
            }
            "photara.history.run-outcome" => {
                if !starts.contains_key(&uuid_field(v, "run_id")?) {
                    return Err(PackageError::Integrity);
                }
            }
            "photara.history.evidence" | "photara.history.receipt" => {
                if !operations.contains_key(&uuid_field(v, "operation_id")?) {
                    return Err(PackageError::Integrity);
                }
                if s.id == "photara.history.receipt"
                    && !attempts.contains_key(&uuid_field(v, "observing_attempt_id")?)
                {
                    return Err(PackageError::Integrity);
                }
            }
            "photara.project.history" => {
                for run in array(v, "runs")? {
                    let record = &c.object(&ref_field(run, "document")?)?.value;
                    require_kind(record, "photara.history.run")?;
                    if run.get("run_id") != record.get("run_id") {
                        return Err(PackageError::Integrity);
                    }
                }
            }
            "photara.history.run" => {
                let start = &c.object(&ref_field(v, "start")?)?.value;
                let outcome = &c.object(&ref_field(v, "outcome")?)?.value;
                require_kind(start, "photara.history.run-start")?;
                require_kind(outcome, "photara.history.run-outcome")?;
                if start.get("run_id") != v.get("run_id")
                    || outcome.get("run_id") != v.get("run_id")
                {
                    return Err(PackageError::Integrity);
                }
                for a in array(v, "attempts")? {
                    for key in ["start", "outcome"] {
                        let record = &c.object(&ref_field(a, key)?)?.value;
                        require_kind(
                            record,
                            if key == "start" {
                                "photara.history.attempt-start"
                            } else {
                                "photara.history.attempt-outcome"
                            },
                        )?;
                        if record.get("attempt_id") != a.get("attempt_id")
                            || record.get("run_id") != v.get("run_id")
                        {
                            return Err(PackageError::Integrity);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
