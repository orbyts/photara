use super::*;

pub(super) fn root(disk: &Disk, reference: &Ref) -> Result<Root> {
    let root: Root = disk.get(reference)?;
    if root.project != id(20000)
        || root.library != id(20001)
        || root.bootstrap != hash(&disk.read("manifest.json")?)
        || root.revision == 0
    {
        return Err(Error::Integrity);
    }
    let mut seen = BTreeSet::new();
    for reference in [
        &root.authored,
        &root.history,
        &root.resources,
        &root.operations,
    ] {
        visit(disk, reference, &mut seen)?;
    }
    if seen != root.inventory {
        return Err(Error::Integrity);
    }
    if !matches!(disk.get::<Object>(&root.authored)?, Object::Graph { .. })
        || !matches!(disk.get::<Object>(&root.history)?, Object::History { .. })
        || !matches!(
            disk.get::<Object>(&root.resources)?,
            Object::Resources { .. }
        )
    {
        return Err(Error::Integrity);
    }
    let Object::Operations { accepted } = disk.get(&root.operations)? else {
        return Err(Error::Integrity);
    };
    let mut operations = BTreeSet::new();
    if accepted.iter().any(|a| {
        a.operation.is_empty()
            || a.intent.is_empty()
            || a.provenance.is_empty()
            || !operations.insert(&a.operation)
    }) {
        return Err(Error::Integrity);
    }
    Ok(root)
}

#[expect(
    clippy::collapsible_match,
    reason = "Keep fallible I/O validation separate from enum selection"
)]
fn visit(disk: &Disk, reference: &Ref, seen: &mut BTreeSet<Ref>) -> Result<()> {
    if !seen.insert(reference.clone()) {
        return Ok(());
    }
    if seen.len() > 256 {
        return Err(Error::Integrity);
    } // fixture budget only
    let object: Object = disk.get(reference)?;
    for child in object.dependencies() {
        visit(disk, &child, seen)?;
    }
    match object {
        Object::Resource { identity, version } => {
            if !matches!(disk.get::<Object>(&version)?, Object::Version {resource, ..} if resource == identity)
            {
                return Err(Error::Integrity);
            }
        }
        Object::Version {
            identity,
            backing,
            provenance,
            retention,
            ..
        } => {
            if !matches!(disk.get::<Object>(&backing)?, Object::Backing { version, receipt, .. } if version == identity && !receipt.is_empty())
                || !matches!(disk.get::<Object>(&retention)?, Object::Retention {version, ..} if version == identity)
                || !matches!(disk.get::<Object>(&provenance)?, Object::Provenance { captured_digest, .. } if captured_digest.len() == 64)
            {
                return Err(Error::Integrity);
            }
        }
        Object::Resources { records } => {
            for child in records {
                if !matches!(disk.get::<Object>(&child)?, Object::Resource { .. }) {
                    return Err(Error::Integrity);
                }
            }
        }
        Object::Graph { snapshot, .. } => {
            if !matches!(disk.get::<Object>(&snapshot)?, Object::Snapshot { .. }) {
                return Err(Error::Integrity);
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn open(disk: &Disk) -> Result<(RootSet, Root)> {
    let head_bytes = disk.read("HEAD.json")?;
    let selected = validate_head(disk, &head_bytes)?;
    if disk.read("HEAD.json")? != head_bytes {
        return Err(Error::Conflict);
    }
    Ok(selected)
}

pub(super) fn validate_head(disk: &Disk, head_bytes: &[u8]) -> Result<(RootSet, Root)> {
    let bootstrap_bytes = disk.read("manifest.json")?;
    let head: Value = parse(head_bytes)?;
    if head["schema"] != json!({"id":"photara.package.head","version":1}) {
        return Err(Error::Unsupported);
    }
    let commit_id = head["commit_id"].as_str().ok_or(Error::Integrity)?;
    if uuid::Uuid::parse_str(commit_id).is_err() || head["project_id"] != id(20000) {
        return Err(Error::Integrity);
    }
    let commit_bytes = disk.read(&format!("commits/{commit_id}.json"))?;
    if head["commit_sha256"] != hash(&commit_bytes) {
        return Err(Error::Integrity);
    }
    let commit: Value = parse(&commit_bytes)?;
    if commit["schema"] != json!({"id":"photara.package.commit","version":1}) {
        return Err(Error::Unsupported);
    }
    let features = commit["required_features"]
        .as_array()
        .ok_or(Error::Unsupported)?;
    let manifest: Value = parse(&disk.read("manifest.json")?)?;
    let mut expected: Vec<Value> = manifest["required_features"]
        .as_array()
        .ok_or(Error::Integrity)?
        .clone();
    expected.push(json!(FEATURE));
    expected.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    if *features != expected {
        return Err(Error::Unsupported);
    }
    let set: RootSet = serde_json::from_value(commit["fixture_root_set"].clone())
        .map_err(|_| Error::Unsupported)?;
    if set.discriminator != DISCRIMINATOR || set.required_reader != "example.fixture.reader-only" {
        return Err(Error::Unsupported);
    }
    if commit["minimum_reader"] != json!({"major":u32::MAX,"minor":0,"fixture_only":true}) {
        return Err(Error::Unsupported);
    }
    if commit["commit_id"] != commit_id
        || commit["project_id"] != id(20000)
        || commit["bootstrap_sha256"] != hash(&disk.read("manifest.json")?)
    {
        return Err(Error::Integrity);
    }
    let active = root(disk, &set.active)?;
    let recovery = root(disk, &set.recovery)?;
    operation_continuity(disk, &active, &recovery)?;
    if commit["package_revision"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .is_none_or(|revision| revision == 0)
        || recovery.revision > active.revision
        || commit["authored"] != legacy_ref(&active.authored)
        || commit["history"] != legacy_ref(&active.history)
        || commit["fixture_operations"] != serde_json::to_value(&active.operations).unwrap()
    {
        return Err(Error::Integrity);
    }
    let inventory_ref = Ref {
        digest: commit["inventory"]["sha256"]
            .as_str()
            .ok_or(Error::Integrity)?
            .into(),
        length: commit["inventory"]["byte_length"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .ok_or(Error::Integrity)?,
    };
    if disk.get::<BTreeSet<Ref>>(&inventory_ref)? != active.inventory {
        return Err(Error::Integrity);
    }
    let mut inventory = active.inventory.clone();
    inventory.extend(recovery.inventory);
    inventory.extend([set.active.clone(), set.recovery.clone()]);
    for reference in &set.pinned {
        inventory.extend(root(disk, reference)?.inventory);
        inventory.insert(reference.clone());
    }
    if inventory != set.inventory {
        return Err(Error::Integrity);
    }
    original_snapshot(disk, &set)?;
    if disk.read("manifest.json")? != bootstrap_bytes {
        return Err(Error::Conflict);
    }
    Ok((set, active))
}

fn operation_continuity(disk: &Disk, active: &Root, recovery: &Root) -> Result<()> {
    let Object::Operations {
        accepted: active_operations,
    } = disk.get(&active.operations)?
    else {
        return Err(Error::Integrity);
    };
    let Object::Operations {
        accepted: recovery_operations,
    } = disk.get(&recovery.operations)?
    else {
        return Err(Error::Integrity);
    };
    if !active_operations.starts_with(&recovery_operations) {
        return Err(Error::Integrity);
    }
    Ok(())
}

fn original_snapshot(disk: &Disk, set: &RootSet) -> Result<()> {
    if disk.read("manifest.json")? != disk.read("fixture-original/manifest.json")? {
        return Err(Error::Integrity);
    }
    // Exact original snapshot, including unknown files, remains separately pinned.
    for (path, expected) in &set.conversion {
        let bytes = disk.read(&format!("fixture-original/{path}"))?;
        if reference(&bytes) != *expected {
            return Err(Error::Integrity);
        }
    }
    if !set.conversion.contains_key("HEAD.json") || !set.conversion.contains_key("manifest.json") {
        return Err(Error::Integrity);
    }
    if file_paths(&disk.path().join("fixture-original"))?
        != set.conversion.keys().cloned().collect()
    {
        return Err(Error::Integrity);
    }
    package::v1_1::validate_directory(
        disk.path().join("fixture-original"),
        PackageLimits::default(),
    )
    .map_err(|_| Error::Integrity)?;
    Ok(())
}

pub(super) fn state(
    disk: &Disk,
    revision: u64,
    predecessor: Option<String>,
    prior: Vec<Accepted>,
) -> Result<Ref> {
    let snapshot = disk.put(&Object::Snapshot {
        authored: "retained-source-and-context".into(),
    })?;
    let authored = disk.put(&Object::Graph {
        revision,
        snapshot: snapshot.clone(),
        opaque: json!({"$ref":"/external/not-a-dependency","extension":"preserved"}),
    })?;
    let history = disk.put(&Object::History {
        retained: vec![snapshot],
    })?;
    let backing = disk.put(&Object::Backing {
        version: "captured-v1".into(),
        byte_length: LARGE,
        storage_location: "logical-storage-location-id".into(),
        opaque_locator: "offline-provider/object-v1".into(),
        receipt: "durably-published-and-verified".into(),
    })?;
    let provenance = disk.put(&Object::Provenance {
        captured_digest: hash(b"synthetic-exact-version"),
        producer: "fixture".into(),
    })?;
    let retention = disk.put(&Object::Retention {
        version: "captured-v1".into(),
        obligations: vec!["authored-reference".into()],
    })?;
    let version = disk.put(&Object::Version {
        resource: "logical-resource".into(),
        identity: "captured-v1".into(),
        backing,
        provenance,
        retention,
    })?;
    let resource = disk.put(&Object::Resource {
        identity: "logical-resource".into(),
        version,
    })?;
    let resources = disk.put(&Object::Resources {
        records: vec![resource],
    })?;
    let mut accepted = prior;
    accepted.push(Accepted {
        operation: format!("op-{revision}"),
        intent: hash(format!("intent-{revision}").as_bytes()),
        resulting_graph: authored.clone(),
        provenance: "original-credential-free-authority".into(),
    });
    let operations = disk.put(&Object::Operations { accepted })?;
    let mut inventory = BTreeSet::new();
    for reference in [&authored, &history, &resources, &operations] {
        visit(disk, reference, &mut inventory)?;
    }
    disk.put(&Root {
        project: id(20000),
        library: id(20001),
        bootstrap: hash(&disk.read("manifest.json")?),
        revision,
        authored,
        history,
        resources,
        operations,
        inventory,
        predecessor,
    })
}

pub(super) fn candidate(disk: &Disk, active: Ref, recovery: Ref) -> Result<Vec<u8>> {
    let (head, commit) = candidate_bytes(disk, active, recovery)?;
    stage_commit(disk, &head, &commit)?;
    Ok(head)
}

pub(super) fn stage_commit(disk: &Disk, head_bytes: &[u8], commit: &[u8]) -> Result<()> {
    let head: Value = parse(head_bytes)?;
    let id = head["commit_id"].as_str().ok_or(Error::Integrity)?;
    if uuid::Uuid::parse_str(id).is_err() || head["commit_sha256"] != hash(commit) {
        return Err(Error::Integrity);
    }
    disk.immutable(&format!("commits/{id}.json"), commit)
}

pub(super) fn candidate_bytes(
    disk: &Disk,
    active: Ref,
    recovery: Ref,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let root = root(disk, &active)?;
    let recovery_root = self::root(disk, &recovery)?;
    let mut inventory = root.inventory.clone();
    inventory.extend(recovery_root.inventory);
    inventory.extend([active.clone(), recovery.clone()]);
    let conversion = disk
        .original
        .iter()
        .map(|(p, b)| (p.clone(), reference(b)))
        .collect();
    let set = RootSet {
        discriminator: DISCRIMINATOR.into(),
        required_reader: "example.fixture.reader-only".into(),
        active,
        recovery,
        pinned: vec![],
        inventory,
        conversion,
    };
    let mut commit: Value = parse(&disk.original[&format!("commits/{}.json", id(20019))])?;
    let current_head: Value = parse(&disk.read("HEAD.json")?)?;
    let current_id = current_head["commit_id"].as_str().ok_or(Error::Integrity)?;
    if uuid::Uuid::parse_str(current_id).is_err() {
        return Err(Error::Integrity);
    }
    let current: Value = parse(&disk.read(&format!("commits/{current_id}.json"))?)?;
    let package_revision = current["package_revision"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .and_then(|r| r.checked_add(1))
        .ok_or(Error::Integrity)?;
    let commit_id = uuid::Uuid::new_v4().to_string();
    commit["commit_id"] = json!(commit_id);
    commit["write_id"] = json!(uuid::Uuid::new_v4().to_string());
    commit["package_revision"] = json!(package_revision.to_string());
    // Sentinel deliberately cannot denote a reserved production reader version.
    commit["minimum_reader"] = json!({"major":u32::MAX,"minor":0,"fixture_only":true});
    commit["fixture_root_set"] = serde_json::to_value(set).unwrap();
    commit["authored"] = legacy_ref(&root.authored);
    commit["history"] = legacy_ref(&root.history);
    commit["inventory"] = legacy_ref(&disk.put(&root.inventory)?);
    commit["fixture_operations"] = serde_json::to_value(root.operations).unwrap();
    let features = commit["required_features"].as_array_mut().unwrap();
    features.push(json!(FEATURE));
    features.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    let bytes = canon(&commit);
    let mut head: Value = parse(&disk.original["HEAD.json"])?;
    head["commit_id"] = json!(commit_id);
    head["commit_sha256"] = json!(hash(&bytes));
    Ok((canon(&head), bytes))
}

fn legacy_ref(reference: &Ref) -> Value {
    json!({"kind":"json","sha256":reference.digest,"byte_length":reference.length.to_string()})
}

pub(super) fn snapshot(disk: &Disk) -> Result<()> {
    for (path, bytes) in &disk.original {
        disk.immutable(&format!("fixture-original/{path}"), bytes)?;
    }
    disk.barrier()
}
