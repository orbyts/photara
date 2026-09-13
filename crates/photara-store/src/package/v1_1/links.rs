use super::*;

pub(super) fn validate_commit(
    commit: &Commit,
    closure: &BTreeSet<ObjectRef>,
    c: &Context,
) -> Result<(), PackageError> {
    let mut required = BTreeSet::new();
    for r in closure.iter().filter(|r| r.kind == ObjectKind::Json) {
        let v = &c.object(r)?.value;
        if known(v)? {
            match kind(v)?.0 {
                "photara.project.authored" | "photara.project.library-snapshot" => {
                    required.insert(FEATURES[0]);
                }
                "photara.project.asset-ledger"
                | "photara.project.resource-ledger"
                | "photara.project.managed-resource"
                | "photara.project.external-resource"
                | "photara.project.representation-content"
                | "photara.history.artifact" => {
                    required.insert(FEATURES[1]);
                }
                "photara.value.asset-set-snapshot"
                | "photara.value.asset-set-page"
                | "photara.value.group-set" => {
                    required.insert(FEATURES[2]);
                }
                "photara.project.saved-graph" | "photara.node.manifest" => {
                    required.insert(FEATURES[4]);
                }
                _ => {
                    required.insert(FEATURES[3]);
                }
            }
        }
    }
    if (!required.is_empty() && commit.minimum_reader.minor < 1)
        || required
            .iter()
            .any(|f| !commit.required_features.iter().any(|x| x == f))
    {
        return Err(PackageError::UnsupportedFeature);
    }
    let a = target(c, &commit.authored, "photara.project.authored")?;
    target(c, &commit.history, "photara.project.history")?;
    if kind(a)?.1 == 2 {
        authored(a, c)?;
    } else {
        super::super::records::validate_authored(&decode(a)?, c)?;
    }
    Ok(())
}
fn list<'a>(
    v: &Value,
    key: &str,
    c: &'a Context,
    kind: &str,
    id_key: &str,
) -> Result<Vec<&'a Value>, PackageError> {
    let values = array(v, key)?
        .iter()
        .map(|r| target(c, &decode(r)?, kind))
        .collect::<Result<Vec<_>, _>>()?;
    let ids = values
        .iter()
        .map(|v| id(v, id_key))
        .collect::<Result<Vec<_>, _>>()?;
    if ids.windows(2).any(|w| w[0] >= w[1]) {
        return Err(PackageError::Record);
    }
    Ok(values)
}
fn authored(record_value: &Value, lookup: &Context) -> Result<(), PackageError> {
    let scope = json!({"kind":"project","project_id":record_value["project_id"]});
    let ctx = target(
        lookup,
        &reference(record_value, "context")?,
        "photara.context.authored",
    )?;
    if field(ctx, "scope")? != &scope {
        return Err(PackageError::Integrity);
    }
    context_owner(ctx, field(record_value, "owning_library_id")?, lookup)?;
    ordered_ids(array(record_value, "graphs")?, "graph_id")?;
    for g in array(record_value, "graphs")? {
        let graph = target(
            lookup,
            &reference(g, "document")?,
            "photara.project.saved-graph",
        )?;
        same(g, graph, &["graph_id"])?;
        if kind(graph)?.1 != 2 {
            return Err(PackageError::UnsupportedVersion);
        }
        let ctx = target(
            lookup,
            &reference(graph, "context")?,
            "photara.context.authored",
        )?;
        if field(ctx, "scope")?
            != &json!({"kind":"graph","project_id":record_value["project_id"],"graph_id":g["graph_id"]})
        {
            return Err(PackageError::Integrity);
        }
        context_owner(ctx, field(record_value, "owning_library_id")?, lookup)?;
        let nodes = array(field(graph, "graph")?, "nodes")?;
        if array(ctx, "node_contexts")?.len() != nodes.len() {
            return Err(PackageError::Integrity);
        }
        for n in array(ctx, "node_contexts")? {
            if !nodes
                .iter()
                .any(|referent_value| referent_value.get("id") == n.get("node_id"))
            {
                return Err(PackageError::Integrity);
            }
            let nc = target(
                lookup,
                &reference(n, "context")?,
                "photara.context.authored",
            )?;
            if field(nc, "scope")?
                != &json!({"kind":"node","project_id":record_value["project_id"],"graph_id":g["graph_id"],"node_id":n["node_id"]})
            {
                return Err(PackageError::Integrity);
            }
            context_owner(nc, field(record_value, "owning_library_id")?, lookup)?;
        }
    }
    let ledger = target(
        lookup,
        &reference(record_value, "asset_ledger")?,
        "photara.project.asset-ledger",
    )?;
    let resources = target(
        lookup,
        &reference(record_value, "resource_ledger")?,
        "photara.project.resource-ledger",
    )?;
    for asset in array(ledger, "assets")? {
        for rep in array(asset, "representations")? {
            let content = target(
                lookup,
                &reference(rep, "current_content")?,
                "photara.project.representation-content",
            )?;
            same(asset, content, &["asset_id"])?;
            same(rep, content, &["representation_id"])?;
            let binding_value = field(content, "binding")?;
            let (key, reference_value) = if text(binding_value, "kind")? == "managed" {
                ("managed_resources", reference(binding_value, "version")?)
            } else {
                ("external_resources", reference(binding_value, "revision")?)
            };
            if !array(resources, key)?.iter().any(|referent_value| {
                decode::<ObjectRef>(referent_value).ok() == Some(reference_value.clone())
            }) {
                return Err(PackageError::Integrity);
            }
            if let Some(retained) = rep.get("retained_content") {
                for reference_value in retained.as_array().ok_or(PackageError::Record)? {
                    let referent_value = target(
                        lookup,
                        &decode(reference_value)?,
                        "photara.project.representation-content",
                    )?;
                    same(content, referent_value, &["asset_id", "representation_id"])?;
                }
            }
        }
    }
    Ok(())
}
fn context_owner(v: &Value, library: &Value, c: &Context) -> Result<(), PackageError> {
    for (key, k) in [
        ("variables", "photara.context.variable"),
        ("expressions", "photara.context.expression"),
        ("captures", "photara.context.snapshot"),
    ] {
        for r in array(v, key)? {
            let x = target(c, &decode(r)?, k)?;
            let l = if key == "expressions" {
                field(field(x, "environment")?, "owning_library_id")?
            } else {
                field(x, "owning_library_id")?
            };
            if l != library {
                return Err(PackageError::Integrity);
            }
        }
    }
    Ok(())
}
pub(super) fn asset_set(
    v: &Value,
    c: &Context,
) -> Result<ct::asset_set::AssetSetSnapshot, PackageError> {
    let mut members = Vec::new();
    for p in array(v, "pages")? {
        let page = target(c, &reference(p, "page")?, "photara.value.asset-set-page")?;
        same(page, v, &["snapshot_id"])?;
        if uint(p, "start_ordinal")? != members.len() as u64
            || uint(page, "start_ordinal")? != members.len() as u64
            || uint(p, "count")? != array(page, "members")?.len() as u64
        {
            return Err(PackageError::Integrity);
        }
        for m in array(page, "members")? {
            let mut member = m.clone();
            member["ordinal"] = json!(members.len());
            let mut metadata = Vec::new();
            for r in array(m, "metadata_refs")? {
                let obs = target(c, &decode(r)?, "photara.metadata.observation")?;
                same(m, obs, &["asset_id"])?;
                metadata.push(json!({"project_id":v["project_id"],"asset_id":m["asset_id"],"target":{"kind":"representation","representation_id":obs["representation_id"],"content_revision_id":obs["content_revision_id"]},"object":r}));
            }
            member.as_object_mut().unwrap().remove("metadata_refs");
            member["metadata"] = json!(metadata);
            for rep in member["representations"]
                .as_array_mut()
                .ok_or(PackageError::Record)?
            {
                rep["project_id"] = v["project_id"].clone();
                rep["asset_id"] = m["asset_id"].clone();
                let content = target(
                    c,
                    &reference(rep, "descriptor")?,
                    "photara.project.representation-content",
                )?;
                same(
                    rep,
                    content,
                    &[
                        "project_id",
                        "asset_id",
                        "representation_id",
                        "content_revision_id",
                    ],
                )?;
            }
            members.push(decode(&member)?);
            if members.len() > ct::asset_set::MAX_MEMBERS {
                return Err(PackageError::Limit);
            }
        }
    }
    if uint(v, "member_count")? != members.len() as u64 {
        return Err(PackageError::Integrity);
    }
    let set = contract(ct::asset_set::AssetSetSnapshot::new(
        decode(field(v, "snapshot_id")?)?,
        decode(field(v, "project_id")?)?,
        members,
    ))?;
    if serde_json::to_value(contract(set.content_digest())?).map_err(|_| PackageError::Record)?
        != v["content_digest"]
    {
        return Err(PackageError::Integrity);
    }
    Ok(set)
}
fn content(record_value: &Value, context: &Context) -> Result<(), PackageError> {
    let binding_value = field(record_value, "binding")?;
    if text(binding_value, "kind")? == "managed" {
        let reference_value = target(
            context,
            &reference(binding_value, "version")?,
            "photara.project.managed-resource",
        )?;
        same(
            binding_value,
            reference_value,
            &["resource_id", "version_id"],
        )?;
        same(record_value, reference_value, &["media"])?;
        let fingerprint_value = field(record_value, "fingerprint")?;
        if text(fingerprint_value, "kind")? == "content-digest"
            && (text(fingerprint_value, "algorithm")? != "sha256"
                || text(fingerprint_value, "value")?
                    != reference(reference_value, "blob")?.sha256.as_str())
        {
            return Err(PackageError::Integrity);
        }
    } else {
        let reference_value = target(
            context,
            &reference(binding_value, "revision")?,
            "photara.project.external-resource",
        )?;
        same(
            binding_value,
            reference_value,
            &["external_ref_id", "revision_id"],
        )?;
        if text(field(reference_value, "content_evidence")?, "kind")? == "sha256"
            && field(record_value, "fingerprint")?.get("value")
                != field(reference_value, "content_evidence")?.get("digest")
        {
            return Err(PackageError::Integrity);
        }
    }
    Ok(())
}
fn authored_context(v: &Value, c: &Context) -> Result<(), PackageError> {
    let vars = list(v, "variables", c, "photara.context.variable", "variable_id")?;
    let mut aggregates = Vec::new();
    for x in vars {
        same(v, x, &["scope"])?;
        aggregates.push(records::variable(x, c)?);
    }
    checked(cx::variable::variable_order(&aggregates))?;
    for x in list(
        v,
        "expressions",
        c,
        "photara.context.expression",
        "expression_id",
    )? {
        if field(x, "owner")? != field(v, "scope")? {
            return Err(PackageError::Integrity);
        }
    }
    list(v, "captures", c, "photara.context.snapshot", "snapshot_id")?;
    for r in array(v, "metadata_selections")? {
        let x = &c.object(&decode(r)?)?.value;
        if ![
            "photara.value.asset-set-snapshot",
            "photara.value.group-set",
            "photara.metadata.patch",
        ]
        .contains(&kind(x)?.0)
        {
            return Err(PackageError::Integrity);
        }
    }
    // Every expression binding is owned by this exact authored context.
    let refs: BTreeSet<ObjectRef> = array(v, "expressions")?
        .iter()
        .map(decode)
        .collect::<Result<_, _>>()?;
    for r in array(v, "variables")? {
        let x = c.object(&decode(r)?)?;
        let mut edges = Vec::new();
        references(&x.value, &mut edges)?;
        for r in edges {
            if r.kind == ObjectKind::Json
                && kind(&c.object(&r)?.value)?.0 == "photara.context.expression"
                && !refs.contains(&r)
            {
                return Err(PackageError::Integrity);
            }
        }
    }
    Ok(())
}
fn groups(v: &Value, c: &Context) -> Result<(), PackageError> {
    let input = target(
        c,
        &reference(v, "input_snapshot")?,
        "photara.value.asset-set-snapshot",
    )?;
    if field(v, "input_digest")? != field(input, "content_digest")? {
        return Err(PackageError::Integrity);
    }
    let set = asset_set(input, c)?;
    for g in array(v, "groups")? {
        text(g, "label")?;
        let ids: Vec<ct::ids::AssetId> = decode(field(g, "member_asset_ids")?)?;
        if ids.windows(2).any(|w| w[0] >= w[1])
            || ids
                .iter()
                .any(|id| !set.members().iter().any(|m| m.asset_id == *id))
        {
            return Err(PackageError::Integrity);
        }
    }
    Ok(())
}
fn patch(v: &Value, c: &Context) -> Result<(), PackageError> {
    let input = target(
        c,
        &reference(v, "input_snapshot")?,
        "photara.value.asset-set-snapshot",
    )?;
    if field(v, "input_digest")? != field(input, "content_digest")? {
        return Err(PackageError::Integrity);
    }
    let set = asset_set(input, c)?;
    let t = field(v, "target")?;
    let ids: Vec<ct::ids::AssetId> = match text(t, "kind")? {
        "asset" => vec![decode(field(t, "asset_id")?)?],
        "subset" => decode(field(t, "member_asset_ids")?)?,
        "all-input" => set.members().iter().map(|m| m.asset_id).collect(),
        "group" => {
            let g = target(
                c,
                &reference(t, "group_snapshot")?,
                "photara.value.group-set",
            )?;
            same(g, v, &["input_snapshot", "input_digest"])?;
            let group = array(g, "groups")?
                .iter()
                .find(|g| g.get("group_id") == t.get("group_id"))
                .ok_or(PackageError::Integrity)?;
            decode(field(group, "member_asset_ids")?)?
        }
        _ => return Err(PackageError::Record),
    };
    if ids.is_empty()
        || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
        || ids
            .iter()
            .any(|id| !set.members().iter().any(|m| m.asset_id == *id))
    {
        return Err(PackageError::Integrity);
    }
    for op in array(v, "operations")? {
        for p in array(op, "preconditions")? {
            let content = target(
                c,
                &reference(p, "descriptor")?,
                "photara.project.representation-content",
            )?;
            same(
                p,
                content,
                &[
                    "asset_id",
                    "representation_id",
                    "content_revision_id",
                    "fingerprint",
                ],
            )?;
            let aid: ct::ids::AssetId = decode(field(p, "asset_id")?)?;
            if !ids.contains(&aid)
                || !set.members().iter().any(|m| {
                    m.asset_id == aid
                        && m.representations.iter().any(|r| {
                            serde_json::to_value(&r.descriptor).ok() == p.get("descriptor").cloned()
                        })
                })
            {
                return Err(PackageError::Integrity);
            }
        }
    }
    Ok(())
}
fn captured_values(
    x: &cx::value::Value,
    c: &Context,
    captures: &[&Value],
) -> Result<(), PackageError> {
    use cx::value::Value as V;
    match x {
        V::AssetSet(d) => {
            let v = captures
                .iter()
                .find(|v| v.get("snapshot_id") == serde_json::to_value(d.snapshot_id).ok().as_ref())
                .ok_or(PackageError::Integrity)?;
            if contract(asset_set(v, c)?.descriptor())? != *d {
                return Err(PackageError::Integrity);
            }
        }
        V::Metadata(m) => {
            for member in &m.spec().members {
                for r in &member.observations {
                    let r: ObjectRef =
                        decode(&serde_json::to_value(r).map_err(|_| PackageError::Record)?)?;
                    let obs = target(c, &r, "photara.metadata.observation")?;
                    if serde_json::to_value(member.asset_id).ok().as_ref() != obs.get("asset_id")
                        || serde_json::to_value(member.representation_id).ok().as_ref()
                            != obs.get("representation_id")
                        || serde_json::to_value(member.content_revision_id)
                            .ok()
                            .as_ref()
                            != obs.get("content_revision_id")
                        || serde_json::to_value(&m.spec().field).ok().as_ref() != obs.get("field")
                    {
                        return Err(PackageError::Integrity);
                    }
                    if let cx::metadata::Observation::Value(value) = &member.outcome {
                        let expected: cx::value::TypedValue = decode(field(obs, "value")?)?;
                        if expected.value != **value || expected.ty != m.spec().value_type {
                            return Err(PackageError::Integrity);
                        }
                    }
                }
            }
        }
        V::List(xs) => {
            for x in xs {
                captured_values(x, c, captures)?;
            }
        }
        V::Record(xs) => {
            for x in xs.values() {
                captured_values(x, c, captures)?;
            }
        }
        V::Optional(Some(x)) => captured_values(x, c, captures)?,
        _ => {}
    }
    Ok(())
}
fn snapshot_links(v: &Value, c: &Context) -> Result<(), PackageError> {
    let snap = records::snapshot(v)?;
    let captures = list(
        v,
        "captures",
        c,
        "photara.value.asset-set-snapshot",
        "snapshot_id",
    )?;
    let expressions = list(
        v,
        "expressions",
        c,
        "photara.context.expression",
        "expression_id",
    )?;
    for f in &snap.spec().entries {
        if let Some(d) = &f.ast_digest
            && !expressions
                .iter()
                .any(|e| e.get("ast_digest") == serde_json::to_value(d).ok().as_ref())
        {
            return Err(PackageError::Integrity);
        }
        if let cx::snapshot::FactValue::Present(x) = &f.value {
            captured_values(x, c, &captures)?;
            let mut dependencies = Vec::new();
            closure::value(x, &mut dependencies)?;
            for r in dependencies
                .into_iter()
                .filter(|r| r.kind == ObjectKind::Json)
            {
                let object = &c.object(&r)?.value;
                if kind(object)?.0 == "photara.metadata.observation" {
                    let sensitivity: cx::snapshot::Sensitivity =
                        decode(field(object, "sensitivity")?)?;
                    let portability: cx::snapshot::Portability =
                        decode(field(object, "portability")?)?;
                    if sensitivity > f.sensitivity || portability > f.portability {
                        return Err(PackageError::Integrity);
                    }
                }
            }
        }
    }
    Ok(())
}
fn graph_contracts(v: &Value, c: &Context) -> Result<(), PackageError> {
    for contract in array(v, "node_contracts")? {
        let node = array(field(v, "graph")?, "nodes")?
            .iter()
            .find(|n| n.get("id") == contract.get("node_id"))
            .ok_or(PackageError::Integrity)?;
        let pin = field(node, "definition")?;
        let required = array(v, "required_packages")?
            .iter()
            .find(|x| {
                x.get("package_id") == pin.get("package_id")
                    && x.get("package_version") == pin.get("package_version")
            })
            .ok_or(PackageError::Integrity)?;
        if required.get("manifest") != contract.get("manifest") {
            return Err(PackageError::Integrity);
        }
        let Some(r) = contract.get("manifest").filter(|r| !r.is_null()) else {
            continue;
        };
        let manifest = target(c, &decode(r)?, "photara.node.manifest")?;
        let m = field(manifest, "manifest")?;
        let node = array(field(v, "graph")?, "nodes")?
            .iter()
            .find(|n| n.get("id") == contract.get("node_id"))
            .ok_or(PackageError::Integrity)?;
        let pin = field(node, "definition")?;
        same(pin, m, &["package_id", "package_version"])?;
        let def = array(m, "definitions")?
            .iter()
            .find(|d| {
                d.get("coordinate").and_then(|x| x.get("definition_id")) == pin.get("definition_id")
            })
            .ok_or(PackageError::Integrity)?;
        same(field(def, "coordinate")?, pin, &["definition_version"])?;
        if digest(&bytes(def)?) != hash(contract, "contract_digest")? {
            return Err(PackageError::Integrity);
        }
        let req = array(v, "required_packages")?
            .iter()
            .find(|x| {
                x.get("package_id") == pin.get("package_id")
                    && x.get("package_version") == pin.get("package_version")
            })
            .ok_or(PackageError::Integrity)?;
        if req.get("manifest") != Some(r) {
            return Err(PackageError::Integrity);
        }
    }
    Ok(())
}

/// In-memory structural projection only. Source objects/bytes are never changed.
struct LegacyView(BTreeMap<Sha256Hex, VerifiedObject>);
impl super::super::ObjectLookup for LegacyView {
    fn objects(&self) -> &BTreeMap<Sha256Hex, VerifiedObject> {
        &self.0
    }
}
fn legacy_view(c: &Context) -> Result<LegacyView, PackageError> {
    let mut out = c.objects.clone();
    for object in out.values_mut() {
        let v = &mut object.value;
        if !known(v)? {
            continue;
        }
        match kind(v)?.0 {
            "photara.project.authored" => {
                v["originating_library_id"] = Value::Null;
                v["asset_inventory"] = v["asset_ledger"].clone();
                v["resource_inventory"] = v["resource_ledger"].clone();
            }
            "photara.project.asset-ledger" => {
                v["schema"]["id"] = json!("photara.project.asset-inventory");
                v["assets"] = json!([]);
            }
            "photara.project.resource-ledger" => {
                let mut rs = Vec::new();
                for r in array(v, "managed_resources")? {
                    let x = target(c, &decode(r)?, "photara.project.managed-resource")?;
                    rs.push(json!({"resource_id":x["resource_id"],"version":"1","blob":x["blob"],"original_filename":"captured"}));
                }
                v["resources"] = json!(rs);
                v["schema"]["id"] = json!("photara.project.resource-inventory");
            }
            "photara.project.representation-content" => {
                let b = field(v, "binding")?.clone();
                if text(&b, "kind")? == "managed" {
                    let r = target(
                        c,
                        &reference(&b, "version")?,
                        "photara.project.managed-resource",
                    )?;
                    v["binding"] = json!({"kind":"managed","resource_id":b["resource_id"],"resource_version":"1","blob":r["blob"],"path":reference(r,"blob")?.path()});
                } else {
                    v["binding"] = json!({"kind":"external","storage_binding_id":b["external_ref_id"],"storage_root_id":null,"relative_path":"unresolved"});
                }
            }
            "photara.project.library-snapshot" => {
                v["source"]["library_id"] = v["source"]["library_id"].clone();
                let r = &v["source"]["revision"];
                v["source"]["revision"] = if r["kind"] == "local" {
                    json!({"authority":"local","value":r["revision"].as_i64().ok_or(PackageError::Record)?.to_string()})
                } else {
                    json!({"authority":"service","value":r["revision"]})
                };
            }
            _ => {}
        }
        v["schema"]["version"] = json!(1);
    }
    Ok(LegacyView(out))
}
#[expect(
    clippy::too_many_lines,
    reason = "Schema-selected cross-object validation"
)]
pub(super) fn validate_links(c: &Context) -> Result<(), PackageError> {
    let mut identities = BTreeMap::new();
    for o in c.objects.values() {
        let v = &o.value;
        if !known(v)? {
            continue;
        }
        let identity = match kind(v)?.0 {
            "photara.project.representation-content" => Some("content_revision_id"),
            "photara.project.managed-resource" => Some("version_id"),
            "photara.project.external-resource" => Some("revision_id"),
            "photara.value.asset-set-snapshot" | "photara.context.snapshot" => Some("snapshot_id"),
            "photara.metadata.observation" => Some("observation_id"),
            "photara.context.change-proposal" => Some("proposal_id"),
            "photara.context.apply-receipt" => Some("receipt_id"),
            _ => None,
        };
        if let Some(key) = identity
            && identities
                .insert((kind(v)?.0, id(v, key)?), &o.reference)
                .is_some()
        {
            return Err(PackageError::Integrity);
        }
        match kind(v)?.0 {
            "photara.project.authored" => authored(v, c)?,
            "photara.project.saved-graph" => graph_contracts(v, c)?,
            "photara.project.representation-content" => content(v, c)?,
            "photara.project.resource-ledger" => {
                list(
                    v,
                    "managed_resources",
                    c,
                    "photara.project.managed-resource",
                    "version_id",
                )?;
                list(
                    v,
                    "external_resources",
                    c,
                    "photara.project.external-resource",
                    "revision_id",
                )?;
                list(v, "artifacts", c, "photara.history.artifact", "artifact_id")?;
            }
            "photara.value.asset-set-snapshot" => {
                asset_set(v, c)?;
            }
            "photara.context.authored" => authored_context(v, c)?,
            "photara.context.variable" => {
                records::variable(v, c)?;
            }
            "photara.context.snapshot" => snapshot_links(v, c)?,
            "photara.metadata.observation" => {
                let content = c
                    .objects
                    .values()
                    .map(|o| &o.value)
                    .find(|x| {
                        kind(x).is_ok_and(|(k, _)| k == "photara.project.representation-content")
                            && x.get("content_revision_id") == v.get("content_revision_id")
                    })
                    .ok_or(PackageError::Integrity)?;
                same(
                    v,
                    content,
                    &[
                        "asset_id",
                        "representation_id",
                        "content_revision_id",
                        "fingerprint",
                    ],
                )?;
            }
            "photara.metadata.patch" => patch(v, c)?,
            "photara.value.group-set" => groups(v, c)?,
            "photara.context.change-proposal" => {
                let s = target(c, &reference(v, "snapshot")?, "photara.context.snapshot")?;
                same(v, s, &["snapshot_id"])?;
                if field(v, "snapshot_digest")? != field(s, "content_digest")? {
                    return Err(PackageError::Integrity);
                }
                let attempt = c
                    .objects
                    .values()
                    .map(|o| &o.value)
                    .find(|x| {
                        kind(x).is_ok_and(|(k, _)| k == "photara.history.attempt-start")
                            && x.get("attempt_id") == v.get("attempt_id")
                    })
                    .ok_or(PackageError::Integrity)?;
                same(v, attempt, &["run_id"])?;
            }
            "photara.context.apply-receipt" => {
                let proposal = c
                    .objects
                    .values()
                    .map(|o| &o.value)
                    .find(|x| {
                        kind(x).is_ok_and(|(k, _)| k == "photara.context.change-proposal")
                            && x.get("operation_id") == v.get("operation_id")
                    })
                    .ok_or(PackageError::Integrity)?;
                let mut spec = proposal.clone();
                spec.as_object_mut().unwrap().remove("schema");
                spec.as_object_mut().unwrap().remove("extensions");
                let proposal: cx::proposal::VariableChangeProposal = decode(&spec)?;
                if serde_json::to_value(checked(proposal.request_digest())?)
                    .map_err(|_| PackageError::Record)?
                    != v["request_digest"]
                {
                    return Err(PackageError::Integrity);
                }
            }
            "photara.history.run-start" => {
                let s = target(
                    c,
                    &reference(v, "context_snapshot")?,
                    "photara.context.snapshot",
                )?;
                same(v, s, &["owning_library_id", "replayability"])?;
                let a = target(
                    c,
                    &reference(v, "source_authored")?,
                    "photara.project.authored",
                )?;
                same(v, a, &["owning_library_id"])?;
                for input in array(v, "input_snapshots")? {
                    target(
                        c,
                        &reference(input, "snapshot")?,
                        "photara.value.asset-set-snapshot",
                    )?;
                    id(input, "node_id")?;
                    let _: ct::schema::LocalName = decode(field(input, "port_id")?)?;
                }
            }
            "photara.history.attempt-start" => {
                for r in array(v, "input_snapshot_refs")? {
                    target(c, &decode(r)?, "photara.value.asset-set-snapshot")?;
                }
                for r in array(v, "operation_refs")? {
                    let op = target(c, &decode(r)?, "photara.history.effect-intent")?;
                    same(v, op, &["run_id", "attempt_id"])?;
                }
            }
            "photara.history.receipt" => {
                let op = c
                    .objects
                    .values()
                    .map(|o| &o.value)
                    .find(|x| {
                        kind(x).is_ok_and(|(k, _)| k == "photara.history.effect-intent")
                            && x.get("operation_id") == v.get("operation_id")
                    })
                    .ok_or(PackageError::Integrity)?;
                same(v, op, &["request_digest"])?;
                for prior in array(v, "prior_receipt_ids")? {
                    let r = c
                        .objects
                        .values()
                        .map(|o| &o.value)
                        .find(|x| {
                            kind(x).is_ok_and(|(k, _)| k == "photara.history.receipt")
                                && x.get("receipt_id") == Some(prior)
                        })
                        .ok_or(PackageError::Integrity)?;
                    same(v, r, &["operation_id", "request_digest"])?;
                    if r.get("receipt_id") == v.get("receipt_id") {
                        return Err(PackageError::Integrity);
                    }
                }
            }
            "photara.history.artifact" => {
                let d = &c.object(&reference(v, "descriptor")?)?.value;
                if ![
                    "photara.project.managed-resource",
                    "photara.project.external-resource",
                ]
                .contains(&kind(d)?.0)
                {
                    return Err(PackageError::Integrity);
                }
            }
            "photara.project.history" => {
                for (key, k, id) in [
                    ("snapshots", "photara.context.snapshot", "snapshot_id"),
                    (
                        "metadata_observations",
                        "photara.metadata.observation",
                        "observation_id",
                    ),
                    (
                        "proposals",
                        "photara.context.change-proposal",
                        "proposal_id",
                    ),
                    (
                        "apply_receipts",
                        "photara.context.apply-receipt",
                        "receipt_id",
                    ),
                    ("artifacts", "photara.history.artifact", "artifact_id"),
                ] {
                    list(v, key, c, k, id)?;
                }
            }
            _ => {}
        }
    }
    let receipts = c
        .objects
        .values()
        .filter(|o| kind(&o.value).is_ok_and(|(k, _)| k == "photara.context.apply-receipt"))
        .map(|o| decode::<cx::proposal::ApplyReceipt>(&payload(&o.value)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut grouped: BTreeMap<ct::ids::OperationId, Vec<cx::proposal::ApplyReceipt>> =
        BTreeMap::new();
    for r in receipts {
        grouped.entry(r.spec().operation_id).or_default().push(r);
    }
    for rs in grouped.values() {
        checked(cx::proposal::summarize_receipts(
            rs[0].spec().operation_id,
            rs[0].spec().request_digest,
            rs,
        ))?;
    }
    let legacy = legacy_view(c)?;
    for o in legacy.0.values() {
        if matches!(
            kind(&o.value)?.0,
            "photara.project.representation-content" | "photara.project.library-snapshot"
        ) {
            super::super::records::validate_record(&o.value, c.project_id, &mut Vec::new())?;
        }
    }
    for o in c.objects.values() {
        if kind(&o.value)?.0 == "photara.project.authored" {
            let a: AuthoredProject = decode(&legacy.object(&o.reference)?.value)?;
            super::super::records::validate_authored(&a, &legacy)?;
        }
    }
    super::super::records::validate_links(&legacy)
}
