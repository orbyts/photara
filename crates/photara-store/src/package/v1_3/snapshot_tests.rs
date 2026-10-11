//! Shared snapshot readers on the existing integrated corpus; no filesystem effects.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::super::{SelectedEnvelope, audit_operations};
use super::*;
struct Records(BTreeMap<ObjectRef, Value>);
impl LogicalRecords for Records {
    fn resolve(&self, object: &ObjectRef, membership: Membership) -> Result<Value, PackageError> {
        if membership != Membership::Semantic {
            return Err(PackageError::Record);
        }
        self.0.get(object).cloned().ok_or(PackageError::Integrity)
    }
}
fn limits() -> TreeLimits {
    TreeLimits {
        max_depth: 16,
        max_pages: 512,
        max_entries: 4096,
        max_leaf_entries: 64,
        max_branch_children: 8,
    }
}
fn json_limits() -> JsonLimits {
    JsonLimits {
        max_bytes: 1 << 20,
        max_depth: 64,
        max_members: 4096,
        max_array_elements: 4096,
    }
}
fn corpus() -> (Records, Value, Value, Value) {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../docs/architecture/proposals/ps2/integrated/linked.json"
    ))
    .unwrap();
    let records = Records(
        data["records"]
            .as_object()
            .unwrap()
            .values()
            .map(|row| {
                let value: Value =
                    serde_json::from_str(row["canonical"].as_str().unwrap()).unwrap();
                (object_ref(&value).unwrap(), value)
            })
            .collect(),
    );
    let commit: Value =
        serde_json::from_str(data["bootstrap"]["commit"].as_str().unwrap()).unwrap();
    let state = records
        .resolve(
            &json_ref(&commit["root_set"]["active"]).unwrap(),
            Membership::Semantic,
        )
        .unwrap();
    (records, data, commit, state)
}
fn identity(state: &Value) -> SelectionIdentity {
    SelectionIdentity {
        project: nonnil(&state["project_id"]).unwrap(),
        library: nonnil(&state["library_id"]).unwrap(),
        bootstrap_sha256: sha(&state["bootstrap_sha256"]).unwrap(),
    }
}
fn encoded(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
fn base(records: &Records, commit: &Value, state: &Value) -> Value {
    let index = records
        .resolve(
            &json_ref(&state["operation_index"]).unwrap(),
            Membership::Semantic,
        )
        .unwrap();
    json!({"schema":{"id":"photara.package.acceptance-base","version":1},"project_id":state["project_id"],"library_id":state["library_id"],"bootstrap_sha256":state["bootstrap_sha256"],"extensions":{},"checkpoint_id":"96000000-0000-4000-8000-000000009901","source_commit":{"commit_id":commit["commit_id"],"commit_sha256":hash_value(commit).unwrap()},"source_state":object_ref(state).unwrap(),"authored_revision":state["authored_revision"],"authored_sha256":state["authored"]["sha256"],"accepted":state["accepted"],"journal_inclusion":state["journal_inclusion"],"receipts_by_id":index["by_id"],"receipts_by_ordinal":index["by_ordinal"],"receipt_count":state["accepted"]["through_ordinal"]})
}
fn suffix(records: &mut Records, old: &Value, base: &Value, receipts: &[Value]) -> Value {
    let mut refs = Vec::new();
    for receipt in receipts {
        let r = object_ref(receipt).unwrap();
        records.0.insert(r.clone(), receipt.clone());
        refs.push(json!({"operation_id":receipt["operation_id"],"request_sha256":receipt["request_sha256"],"acceptance_ordinal":receipt["acceptance_ordinal"],"receipt":r}));
    }
    let mut roots = Vec::new();
    for (kind, key) in [
        ("operation-id", "operation_id"),
        ("operation-ordinal", "acceptance_ordinal"),
    ] {
        refs.sort_by_key(|entry| entry[key].as_str().unwrap().to_owned());
        let leaf = json!({"schema":{"id":format!("photara.package.{kind}-leaf"),"version":1},"project_id":old["project_id"],"extensions":{},"count":refs.len().to_string(),"entries":refs});
        let r = object_ref(&leaf).unwrap();
        records.0.insert(r.clone(), leaf);
        roots.push(r);
    }
    let base_ref = object_ref(base).unwrap();
    records.0.insert(base_ref.clone(), base.clone());
    let index = json!({"schema":{"id":"photara.package.operation-index","version":3},"project_id":old["project_id"],"library_id":old["library_id"],"bootstrap_sha256":old["bootstrap_sha256"],"extensions":{},"base":base_ref,"accepted":old["accepted"],"by_id":roots[0],"by_ordinal":roots[1]});
    let r = object_ref(&index).unwrap();
    records.0.insert(r.clone(), index);
    let mut state = old.clone();
    state["schema"]["version"] = json!(3);
    state["operation_index"] = serde_json::to_value(r).unwrap();
    state
}
#[test]
fn snapshot_adoption_empty_suffix_scoped_original_retry_and_corruption() {
    let (mut records, _, commit, state) = corpus();
    let expected = identity(&state);
    let proof = audit_operations(&records, &state, &expected, limits()).unwrap();
    let value = base(&records, &commit, &state);
    assert_eq!(encoded(&value).len(), 1424);
    assert_eq!(
        hash_value(&value).unwrap().as_str(),
        "8f6944120c4526f0a206489a9d8891d98bd50f27b1705d8d4b1ad42bd11a1d8e"
    );
    let base = AcceptanceBase::parse(&encoded(&value), &expected, json_limits()).unwrap();
    base.audit_adoption(&records, &commit, &state, &expected, limits())
        .unwrap();
    let newest = proof.receipts().last().unwrap();
    let operation = nonnil(&newest["operation_id"]).unwrap();
    let request = sha(&newest["request_sha256"]).unwrap();
    assert_eq!(
        base.lookup(&records, &expected, operation, Some(&request), limits())
            .unwrap()
            .as_ref(),
        Some(newest)
    );
    assert!(
        base.lookup(
            &records,
            &expected,
            operation,
            Some(&sha(&json!("0".repeat(64))).unwrap()),
            limits()
        )
        .is_err()
    );
    let new = suffix(&mut records, &state, &value, &[]);
    let empty = Records(BTreeMap::new());
    let audit = audit_snapshot_operations(&records, &empty, &new, &expected, limits()).unwrap();
    assert!(audit.receipts().is_empty());
    assert!(
        !audit
            .objects()
            .contains(&json_ref(&value["receipts_by_id"]).unwrap())
    );
    assert!(
        base.lookup(&empty, &expected, operation, Some(&request), limits())
            .is_err()
    );
    let mut changed = new.clone();
    changed["authored_revision"] = json!("999");
    assert!(audit_snapshot_operations(&records, &empty, &changed, &expected, limits()).is_err());
    let mut changed = value.clone();
    changed["extensions"] = json!({"test.extension":1});
    assert!(AcceptanceBase::parse(&encoded(&changed), &expected, json_limits()).is_err());
    let mut changed = value.clone();
    changed["receipt_count"] = json!("2");
    assert!(AcceptanceBase::parse(&encoded(&changed), &expected, json_limits()).is_err());
    let mut changed = value.clone();
    changed["source_state"] = serde_json::to_value(object_ref(&commit).unwrap()).unwrap();
    assert!(
        AcceptanceBase::parse(&encoded(&changed), &expected, json_limits())
            .unwrap()
            .audit_adoption(&records, &commit, &state, &expected, limits())
            .is_err()
    );
    let mut tight = json_limits();
    tight.max_bytes = 1;
    assert!(AcceptanceBase::parse(&encoded(&value), &expected, tight).is_err());
}
#[test]
fn snapshot_suffix_replays_existing_receipts_and_refuses_gaps_or_archive_reuse() {
    let (mut records, _, commit, state) = corpus();
    let expected = identity(&state);
    let full = audit_operations(&records, &state, &expected, limits()).unwrap();
    let previous = records
        .0
        .values()
        .find(|v| {
            v["schema"]["id"] == "photara.package.state-root"
                && v["accepted"]["through_ordinal"] == "2"
        })
        .unwrap()
        .clone();
    let value = base(&records, &commit, &previous);
    let next = suffix(&mut records, &state, &value, &full.receipts()[2..]);
    let proof = audit_snapshot_operations(&records, &records, &next, &expected, limits()).unwrap();
    assert_eq!(proof.receipts(), &full.receipts()[2..]);
    let mut tight = limits();
    tight.max_entries = 0;
    assert!(audit_snapshot_operations(&records, &records, &next, &expected, tight).is_err());
    let mut changed = full.receipts()[2].clone();
    changed["before"]["revision"] = json!("0");
    let wrong = suffix(&mut records, &state, &value, &[changed]);
    assert!(audit_snapshot_operations(&records, &records, &wrong, &expected, limits()).is_err());
    let wrong = suffix(&mut records, &state, &value, &full.receipts()[1..]);
    assert!(audit_snapshot_operations(&records, &records, &wrong, &expected, limits()).is_err());
    let mut changed = full.receipts()[2].clone();
    changed["operation_id"] = full.receipts()[0]["operation_id"].clone();
    let wrong = suffix(&mut records, &state, &value, &[changed]);
    assert!(audit_snapshot_operations(&records, &records, &wrong, &expected, limits()).is_err());
}
#[test]
fn snapshot_floor_feature_and_root_dispatch_preserve_bootstrap() {
    let (_, data, mut commit, state) = corpus();
    let manifest = data["bootstrap"]["manifest"].as_str().unwrap().as_bytes();
    let mut head: Value =
        serde_json::from_str(data["bootstrap"]["head"].as_str().unwrap()).unwrap();
    SelectedEnvelope::parse(
        manifest,
        &encoded(&head),
        &encoded(&commit),
        identity(&state),
        json_limits(),
    )
    .unwrap();
    commit["minimum_reader"] = json!({"major":1,"minor":4});
    head["commit_sha256"] = json!(hash_value(&commit).unwrap());
    assert!(
        SelectedEnvelope::parse(
            manifest,
            &encoded(&head),
            &encoded(&commit),
            identity(&state),
            json_limits()
        )
        .is_err()
    );
    commit["required_features"]
        .as_array_mut()
        .unwrap()
        .push(json!(FEATURE));
    commit["required_features"]
        .as_array_mut()
        .unwrap()
        .sort_by_key(|v| v.as_str().unwrap().to_owned());
    commit["root_set"]["schema"]["version"] = json!(3);
    commit["root_set"]["archive_placement"] = json!({"locator":commit["root_set"]["placement"]["root_placements"],"ownership":commit["root_set"]["placement"]["accounting"]});
    head["commit_sha256"] = json!(hash_value(&commit).unwrap());
    SelectedEnvelope::parse(
        manifest,
        &encoded(&head),
        &encoded(&commit),
        identity(&state),
        json_limits(),
    )
    .unwrap();
    commit["minimum_reader"] = json!({"major":1,"minor":3});
    head["commit_sha256"] = json!(hash_value(&commit).unwrap());
    assert!(
        SelectedEnvelope::parse(
            manifest,
            &encoded(&head),
            &encoded(&commit),
            identity(&state),
            json_limits()
        )
        .is_err()
    );
}

#[test]
fn snapshot_selected_allocation_authenticates_requested_ranges_and_budgets() {
    use super::super::{
        Arena, FrameKind, FrameLimits, PackedAllocation, PhysicalRef, SelectedAllocation,
        encode_frame,
    };
    use crate::package::{DecimalU64, digest};
    let (records, _, commit, state) = corpus();
    let body = encoded(&base(&records, &commit, &state));
    let frame_limits = FrameLimits {
        json: json_limits(),
        max_allocation_bytes: 1 << 20,
        max_frames: 8,
    };
    let frame = encode_frame(Arena::Data, FrameKind::Semantic, &body, frame_limits).unwrap();
    let id = nonnil(&state["project_id"]).unwrap();
    let reference = PhysicalRef {
        allocation_id: id,
        arena: Arena::Data,
        offset: DecimalU64::parse("0").unwrap(),
        byte_length: DecimalU64::parse(&frame.len().to_string()).unwrap(),
        record_sha256: digest(&frame),
    };
    let mut bytes = frame.clone();
    bytes.extend_from_slice(&frame);
    bytes[frame.len() + 16] = b'['; // Corrupt historical body within actual full extent.
    let selected = SelectedAllocation::open(id, Arena::Data, &bytes, frame_limits).unwrap();
    assert_eq!(
        selected.resolve(&reference, FrameKind::Semantic).unwrap(),
        base(&records, &commit, &state)
    );
    let mut archive = reference.clone();
    archive.offset = DecimalU64::parse(&frame.len().to_string()).unwrap();
    assert!(selected.resolve(&archive, FrameKind::Semantic).is_err());
    let old = PackedAllocation::open(id, Arena::Data, &bytes, frame_limits).unwrap();
    assert!(old.resolve(&archive, FrameKind::Semantic).is_err());
    let mut overlap = reference.clone();
    overlap.offset = DecimalU64::parse("1").unwrap();
    assert!(selected.resolve(&overlap, FrameKind::Semantic).is_err());
    let mut changed = reference.clone();
    changed.record_sha256 = digest(b"changed");
    assert!(selected.resolve(&changed, FrameKind::Semantic).is_err());
    let mut changed = reference.clone();
    changed.byte_length = DecimalU64::parse(&(bytes.len() + 1).to_string()).unwrap();
    assert!(
        SelectedAllocation::open(id, Arena::Data, &bytes, frame_limits)
            .unwrap()
            .resolve(&changed, FrameKind::Semantic)
            .is_err()
    );
    assert!(
        selected
            .resolve_json(
                &reference,
                FrameKind::Semantic,
                &object_ref(&commit).unwrap()
            )
            .is_err()
    );
    let mut tight = frame_limits;
    tight.max_frames = 0;
    assert!(
        SelectedAllocation::open(id, Arena::Data, &bytes, tight)
            .unwrap()
            .resolve(&reference, FrameKind::Semantic)
            .is_err()
    );
    tight = frame_limits;
    tight.json.max_bytes = 1;
    assert!(
        SelectedAllocation::open(id, Arena::Data, &bytes, tight)
            .unwrap()
            .resolve(&reference, FrameKind::Semantic)
            .is_err()
    );
    tight = frame_limits;
    tight.max_allocation_bytes = bytes.len() - 1;
    assert!(SelectedAllocation::open(id, Arena::Data, &bytes, tight).is_err());
}
