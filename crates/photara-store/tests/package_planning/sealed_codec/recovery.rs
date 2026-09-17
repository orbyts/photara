use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub old_head: Vec<u8>,
    pub next_head: Vec<u8>,
    pub commit_bytes: Vec<u8>,
    pub operation: String,
    pub request: String,
    pub inclusion: Ref,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Pending,
    Acknowledged,
}

pub(super) fn path(operation: &str, kind: &str) -> Result<String> {
    if operation.is_empty()
        || !operation
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(Error::Conflict);
    }
    Ok(format!("fixture-journal/{operation}-{kind}.json"))
}

pub(super) fn prepare(disk: &Disk, active: Ref, recovery: Ref) -> Result<Intent> {
    codec::snapshot(disk)?;
    let state = codec::root(disk, &active)?;
    let Object::Operations { accepted } = disk.get(&state.operations)? else {
        return Err(Error::Integrity);
    };
    let accepted = accepted.last().ok_or(Error::Integrity)?;
    if disk
        .path()
        .join(path(&accepted.operation, "intent")?)
        .exists()
    {
        let original = read_intent(disk, &accepted.operation)?;
        if original.request != accepted.intent || original.inclusion != state.operations {
            return Err(Error::Conflict);
        }
        codec::stage_commit(disk, &original.next_head, &original.commit_bytes)?;
        return Ok(original);
    }
    let (next_head, commit_bytes) = codec::candidate_bytes(disk, active, recovery)?;
    let intent = Intent {
        old_head: disk.read("HEAD.json")?,
        next_head,
        commit_bytes,
        operation: accepted.operation.clone(),
        request: accepted.intent.clone(),
        inclusion: state.operations,
    };
    disk.write(
        &path(&intent.operation, "intent")?,
        &canonical(&json!({"intent":intent,"digest":hash(&canonical(&intent)?)}))?,
    )?;
    codec::stage_commit(disk, &intent.next_head, &intent.commit_bytes)?;
    Ok(intent)
}

pub(super) fn reconcile(
    disk: &Disk,
    operation: &str,
    request: &str,
    barrier_known: bool,
) -> Result<Outcome> {
    let intent = read_intent(disk, operation)?;
    if intent.operation != operation || intent.request != request {
        return Err(Error::Conflict);
    }
    let head = disk.read("HEAD.json")?;
    if head == intent.old_head {
        if disk.path().join(path(operation, "receipt")?).exists() {
            return Err(Error::Conflict);
        }
        return Ok(Outcome::Pending);
    }
    if head != intent.next_head {
        return Err(Error::Conflict);
    }
    let (_, root) = codec::open(disk)?;
    if root.operations != intent.inclusion {
        return Err(Error::Integrity);
    }
    let Object::Operations { accepted } = disk.get(&root.operations)? else {
        return Err(Error::Integrity);
    };
    if !accepted.iter().any(|a| {
        a.operation == operation && a.intent == request && a.resulting_graph == root.authored
    }) {
        return Err(Error::Integrity);
    }
    if !barrier_known {
        return Err(Error::Unknown);
    }
    disk.barrier()?;
    // Same immutable acknowledgement is reconstructed after a missing/unknown
    // receipt; no new write/operation IDs or replay of the authored mutation.
    disk.immutable(&path(operation,"receipt")?,&canonical(&json!({"operation":operation,"request":request,"head":hash(&head),"inclusion":intent.inclusion}))?)?;
    Ok(Outcome::Acknowledged)
}

pub(super) fn publish(disk: &Disk, intent: &Intent) -> Result<()> {
    if disk.read("HEAD.json")? != intent.old_head {
        return Err(Error::Conflict);
    }
    codec::stage_commit(disk, &intent.next_head, &intent.commit_bytes)?;
    let old_head: Value = parse(&intent.old_head)?;
    let old_id = old_head["commit_id"].as_str().ok_or(Error::Integrity)?;
    if uuid::Uuid::parse_str(old_id).is_err() {
        return Err(Error::Integrity);
    }
    let old: Value = parse(&disk.read(&format!("commits/{old_id}.json"))?)?;
    let next: Value = parse(&intent.commit_bytes)?;
    let old_revision = old["package_revision"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or(Error::Integrity)?;
    let next_revision = next["package_revision"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or(Error::Integrity)?;
    if old_revision.checked_add(1) != Some(next_revision) {
        return Err(Error::Integrity);
    }
    codec::validate_head(disk, &intent.next_head)?;
    if disk.read("HEAD.json")? != intent.old_head {
        return Err(Error::Conflict);
    }
    disk.replace_head(&intent.next_head)
}

fn read_intent(disk: &Disk, operation: &str) -> Result<Intent> {
    let envelope: Value = parse(
        &disk
            .read(&path(operation, "intent")?)
            .map_err(|_| Error::Conflict)?,
    )?;
    if envelope["digest"] != hash(&canon(&envelope["intent"])) {
        return Err(Error::Integrity);
    }
    serde_json::from_value(envelope["intent"].clone()).map_err(|_| Error::Integrity)
}
