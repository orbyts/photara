//! Review-only snapshot-recovery candidates. No production dispatch or admission.
//! Empty suffix/adoption and selected frame obligations only; not a turnover writer.
use super::*;
use std::collections::BTreeSet;
use wire::{ensure, key, number, parse, uuid};
type Result<T> = wire::Result<T>;

fn fields(value: &Value, expected: &[&str]) -> Result<()> {
    let object = value.as_object().ok_or("object")?;
    ensure(
        object.len() == expected.len() && expected.iter().all(|k| object.contains_key(*k)),
        "fields",
    )
}
fn schema(value: &Value, name: &str, version: u64) -> Result<()> {
    ensure(
        value["schema"] == json!({"id":name,"version":version}),
        "schema dispatch",
    )?;
    ensure(value["extensions"] == json!({}), "extensions")
}
fn common(value: &Value, state: &Value) -> Result<()> {
    for field in ["project_id", "library_id", "bootstrap_sha256"] {
        ensure(value[field] == state[field], "identity")?;
    }
    Ok(())
}
struct Candidate {
    state: Value,
    commit: Value,
    base: Value,
    index: Value,
    leaves: [Value; 2],
}
fn candidate() -> Candidate {
    let fixture = support::Fixture::load("integrated");
    let commit: Value =
        serde_json::from_str(fixture.data["bootstrap"]["commit"].as_str().unwrap()).unwrap();
    let get = |r: &Value| -> Value {
        serde_json::from_str(
            fixture.data["records"][r["sha256"].as_str().unwrap()]["canonical"]
                .as_str()
                .unwrap(),
        )
        .unwrap()
    };
    let state = get(&commit["root_set"]["active"]);
    let previous = get(&state["operation_index"]);
    let mut base = json!({"schema":{"id":"photara.package.acceptance-base","version":1},"extensions":{},
        "checkpoint_id":"96000000-0000-4000-8000-000000009901",
        "source_commit":{"commit_id":commit["commit_id"],"commit_sha256":hash(&encode(&commit))},
        "source_state":reference(&state),"authored_revision":state["authored_revision"],"authored_sha256":state["authored"]["sha256"],
        "accepted":state["accepted"],"journal_inclusion":state["journal_inclusion"],
        "receipts_by_id":previous["by_id"],"receipts_by_ordinal":previous["by_ordinal"],"receipt_count":state["accepted"]["through_ordinal"]});
    for field in ["project_id", "library_id", "bootstrap_sha256"] {
        base[field] = state[field].clone();
    }
    let leaves = ["operation-id", "operation-ordinal"].map(|kind| json!({"schema":{"id":format!("photara.package.{kind}-leaf"),"version":1},"project_id":state["project_id"],"count":"0","entries":[],"extensions":{}}));
    let mut index = json!({"schema":{"id":"photara.package.operation-index","version":3},"extensions":{},"base":reference(&base),"accepted":base["accepted"],"by_id":reference(&leaves[0]),"by_ordinal":reference(&leaves[1])});
    for field in ["project_id", "library_id", "bootstrap_sha256"] {
        index[field] = state[field].clone();
    }
    Candidate {
        state,
        commit,
        base,
        index,
        leaves,
    }
}
fn validate_adoption(bytes: &[u8], candidate: &Candidate) -> Result<Value> {
    let base = parse(bytes)?;
    fields(
        &base,
        &[
            "schema",
            "project_id",
            "library_id",
            "bootstrap_sha256",
            "extensions",
            "checkpoint_id",
            "source_commit",
            "source_state",
            "authored_revision",
            "authored_sha256",
            "accepted",
            "journal_inclusion",
            "receipts_by_id",
            "receipts_by_ordinal",
            "receipt_count",
        ],
    )?;
    schema(&base, "photara.package.acceptance-base", 1)?;
    common(&base, &candidate.state)?;
    uuid(&base["checkpoint_id"])?;
    for field in ["source_state", "receipts_by_id", "receipts_by_ordinal"] {
        key(&base[field])?;
    }
    ensure(
        base["source_state"] == reference(&candidate.state),
        "source state",
    )?;
    ensure(
        base["source_commit"]
            == json!({"commit_id":candidate.commit["commit_id"],"commit_sha256":hash(&encode(&candidate.commit))}),
        "source commit",
    )?;
    for field in ["authored_revision", "accepted", "journal_inclusion"] {
        ensure(base[field] == candidate.state[field], "adopted coordinate")?;
    }
    ensure(
        base["authored_sha256"] == candidate.state["authored"]["sha256"],
        "authored digest",
    )?;
    ensure(
        number(&base["receipt_count"])? == number(&base["accepted"]["through_ordinal"])?,
        "coverage",
    )?;
    // Exact already-audited corpus roots; this test does not replace a full adoption audit.
    ensure(
        base["receipts_by_id"] == candidate.base["receipts_by_id"]
            && base["receipts_by_ordinal"] == candidate.base["receipts_by_ordinal"],
        "adopted indexes",
    )?;
    Ok(base)
}
fn validate_empty_suffix(bytes: &[u8], candidate: &Candidate) -> Result<()> {
    let index = parse(bytes)?;
    fields(
        &index,
        &[
            "schema",
            "project_id",
            "library_id",
            "bootstrap_sha256",
            "extensions",
            "base",
            "accepted",
            "by_id",
            "by_ordinal",
        ],
    )?;
    schema(&index, "photara.package.operation-index", 3)?;
    common(&index, &candidate.state)?;
    ensure(index["base"] == reference(&candidate.base), "base link")?;
    ensure(
        index["accepted"] == candidate.base["accepted"],
        "empty suffix coordinate",
    )?;
    for (field, leaf) in ["by_id", "by_ordinal"].into_iter().zip(&candidate.leaves) {
        ensure(index[field] == reference(leaf), "suffix root")?;
    }
    Ok(())
}
#[test]
fn checkpoint_candidate_exact_adoption_and_empty_suffix() {
    let candidate = candidate();
    let vectors = [
        (
            &candidate.base,
            1424,
            "8f6944120c4526f0a206489a9d8891d98bd50f27b1705d8d4b1ad42bd11a1d8e",
        ),
        (
            &candidate.leaves[0],
            158,
            "5913c637d24acc366d39a4c04c37716dffa4db028ebea70bba9e7b6b0b7313b4",
        ),
        (
            &candidate.leaves[1],
            163,
            "ce4323a8968a9ad3cac51a15d7c76c86a3286ea72493813184eb5a83ab67af32",
        ),
        (
            &candidate.index,
            752,
            "31d45052d27bded73f69aeea28337fce97e1df69ac81e3946ba7f2cb3e26bab1",
        ),
    ];
    for (value, length, digest) in vectors {
        assert_eq!(
            (encode(value).len(), hash(&encode(value))),
            (length, digest.into())
        );
    }
    validate_adoption(&encode(&candidate.base), &candidate).unwrap();
    validate_empty_suffix(&encode(&candidate.index), &candidate).unwrap();
    for (field, replacement) in [
        ("project_id", json!(uid(1))),
        ("library_id", json!(uid(2))),
        ("bootstrap_sha256", json!("0".repeat(64))),
        ("receipt_count", json!("2")),
        ("extensions", json!({"unknown":true})),
        (
            "receipts_by_id",
            candidate.base["receipts_by_ordinal"].clone(),
        ),
        ("source_state", reference(&candidate.commit)),
    ] {
        let mut changed = candidate.base.clone();
        changed[field] = replacement;
        assert!(
            validate_adoption(&encode(&changed), &candidate).is_err(),
            "{field}"
        );
    }
    let mut noncanonical = encode(&candidate.base);
    noncanonical.push(b' ');
    assert!(validate_adoption(&noncanonical, &candidate).is_err());
    for (field, replacement) in [
        (
            "schema",
            json!({"id":"photara.package.operation-index","version":2}),
        ),
        ("base", reference(&candidate.state)),
        (
            "accepted",
            json!({"through_ordinal":"0","prefix_sha256":"0".repeat(64)}),
        ),
        ("by_id", reference(&candidate.leaves[1])),
        ("extra", Value::Null),
    ] {
        let mut changed = candidate.index.clone();
        changed[field] = replacement;
        assert!(
            validate_empty_suffix(&encode(&changed), &candidate).is_err(),
            "{field}"
        );
    }
}

/// Test-only selected-record obligation. Each supplied physical coordinate is
/// authenticated by the caller's typed locator; no whole-prefix audit is asserted.
fn selected_frames(
    claim: &Value,
    bytes: &[u8],
    expected: &Value,
    records: &[(v1_3::PhysicalRef, Value)],
) -> Result<()> {
    fields(
        claim,
        &[
            "schema",
            "project_id",
            "extensions",
            "allocation_id",
            "arena",
            "layout",
            "owned_extent",
            "verification",
            "sealed",
            "sealed_charge",
        ],
    )?;
    schema(claim, "photara.storage.allocation-claim", 2)?;
    ensure(claim == expected, "registered claim/charge identity")?;
    ensure(
        claim["layout"] == "framed-json" && claim["verification"] == "selected-records",
        "claim mode",
    )?;
    let extent = number(&claim["owned_extent"])?;
    ensure(extent == bytes.len() as u64, "extent")?;
    let mut spans = Vec::new();
    for (physical, logical) in records {
        ensure(
            physical.allocation_id.to_string() == claim["allocation_id"]
                && physical.arena == v1_3::Arena::Data,
            "physical identity",
        )?;
        let start = physical.offset.get();
        let end = start
            .checked_add(physical.byte_length.get())
            .ok_or("overflow")?;
        ensure(
            end <= extent
                && spans
                    .iter()
                    .all(|&(left, right)| end <= left || start >= right),
            "overlap/extent",
        )?;
        spans.push((start, end));
        let frame = bytes
            .get(
                usize::try_from(start).map_err(|_| "offset")?
                    ..usize::try_from(end).map_err(|_| "end")?,
            )
            .ok_or("range")?;
        let packed =
            v1_3::PackedAllocation::open(physical.allocation_id, physical.arena, frame, frames())
                .map_err(|_| "frame")?;
        let mut local = physical.clone();
        local.offset = package::DecimalU64::parse("0").unwrap();
        let value = packed
            .resolve(&local, v1_3::FrameKind::Semantic)
            .map_err(|_| "physical/canonical")?;
        ensure(reference(&value) == *logical, "logical digest")?;
    }
    Ok(())
}
#[test]
fn checkpoint_candidate_selected_frames_refuse_tampering_overlap_and_identity() {
    let candidate = candidate();
    let body = encode(&candidate.base);
    let frame = v1_3::encode_frame(
        v1_3::Arena::Data,
        v1_3::FrameKind::Semantic,
        &body,
        frames(),
    )
    .unwrap();
    let allocation = package::PackageUuid::parse(&uid(501)).unwrap();
    let physical = v1_3::PhysicalRef {
        allocation_id: allocation,
        arena: v1_3::Arena::Data,
        offset: package::DecimalU64::parse("0").unwrap(),
        byte_length: package::DecimalU64::parse(&frame.len().to_string()).unwrap(),
        record_sha256: support::hash(&frame),
    };
    let claim = json!({"schema":{"id":"photara.storage.allocation-claim","version":2},"project_id":candidate.state["project_id"],"extensions":{},"allocation_id":allocation.to_string(),"arena":"data","layout":"framed-json","owned_extent":frame.len().to_string(),"verification":"selected-records","sealed":false,"sealed_charge":null});
    let entry = (physical.clone(), reference(&candidate.base));
    selected_frames(&claim, &frame, &claim, std::slice::from_ref(&entry)).unwrap();
    assert!(selected_frames(&claim, &frame, &claim, &[entry.clone(), entry.clone()]).is_err());
    let mut changed = physical.clone();
    changed.offset = package::DecimalU64::parse("1").unwrap();
    assert!(selected_frames(&claim, &frame, &claim, &[(changed, entry.1.clone())]).is_err());
    let mut changed = physical.clone();
    changed.allocation_id = package::PackageUuid::parse(&uid(502)).unwrap();
    assert!(selected_frames(&claim, &frame, &claim, &[(changed, entry.1.clone())]).is_err());
    let mut changed = frame.clone();
    changed[16] = b'[';
    assert!(selected_frames(&claim, &changed, &claim, std::slice::from_ref(&entry)).is_err());
    // Correct physical digest cannot make noncanonical JSON acceptable.
    let mut changed = frame.clone();
    changed[17] = b' ';
    let mut changed_ref = physical;
    changed_ref.record_sha256 = support::hash(&changed);
    assert!(selected_frames(&claim, &changed, &claim, &[(changed_ref, entry.1.clone())]).is_err());
    assert!(
        selected_frames(
            &claim,
            &frame,
            &claim,
            &[(entry.0.clone(), reference(&candidate.index))]
        )
        .is_err()
    );
    for (field, replacement) in [
        (
            "schema",
            json!({"id":"photara.storage.allocation-claim","version":1}),
        ),
        ("owned_extent", json!("1")),
        ("project_id", json!(uid(503))),
        ("extensions", json!({"unrecognized":true})),
    ] {
        let mut changed = claim.clone();
        changed[field] = replacement;
        assert!(selected_frames(&changed, &frame, &claim, std::slice::from_ref(&entry)).is_err());
    }
    // Unrequested archive bytes inside the observed extent are not audited.
    let mut archive = frame.clone();
    archive.extend_from_slice(&frame);
    archive[frame.len() + 16] = b'[';
    let mut archive_claim = claim.clone();
    archive_claim["owned_extent"] = json!(archive.len().to_string());
    selected_frames(
        &archive_claim,
        &archive,
        &archive_claim,
        std::slice::from_ref(&entry),
    )
    .unwrap();
    let mut requested_archive = entry.clone();
    requested_archive.0.offset = package::DecimalU64::parse(&frame.len().to_string()).unwrap();
    assert!(
        selected_frames(
            &archive_claim,
            &archive,
            &archive_claim,
            &[entry, requested_archive]
        )
        .is_err()
    );
}

fn acyclic(edges: &[(&str, &[&str])]) -> bool {
    fn visit<'a>(
        node: &'a str,
        edges: &[(&'a str, &[&'a str])],
        active: &mut BTreeSet<&'a str>,
        done: &mut BTreeSet<&'a str>,
    ) -> bool {
        if done.contains(node) {
            return true;
        }
        if !active.insert(node) {
            return false;
        }
        if let Some((_, children)) = edges.iter().find(|(name, _)| *name == node) {
            for child in *children {
                if !visit(child, edges, active, done) {
                    return false;
                }
            }
        }
        active.remove(node);
        done.insert(node);
        true
    }
    let mut done = BTreeSet::new();
    edges
        .iter()
        .all(|(node, _)| visit(node, edges, &mut BTreeSet::new(), &mut done))
}
#[test]
fn checkpoint_candidate_maintenance_local_typed_dependencies_are_acyclic() {
    // Exact proposed reference-bearing fields; source_state/source_commit,
    // previous_sha256 and HEAD parent digests are deliberately NOT traversal edges.
    let edges: &[(&str, &[&str])] = &[
        ("base", &["receipt-id", "receipt-ordinal"]),
        ("index", &["base", "suffix-id", "suffix-ordinal"]),
        ("state", &["index", "authored", "history", "resources"]),
        (
            "original",
            &[
                "old-selector",
                "base",
                "state",
                "recovery",
                "pins",
                "archive-placement",
                "lookup",
                "archive",
            ],
        ),
        ("phase", &["original"]),
        ("commit", &["state", "phase"]),
        ("local-base", &["base", "commit", "lookup", "archive"]),
        ("header", &["local-base"]),
        ("receipt", &["original", "local-base", "commit"]),
        ("selector", &["local-base", "original"]),
    ];
    assert!(acyclic(edges));
    let mut cyclic = edges.to_vec();
    cyclic.push(("authored", &["original"]));
    assert!(!acyclic(&cyclic));
    let mut cyclic = edges.to_vec();
    cyclic.push(("lookup", &["local-base"]));
    assert!(!acyclic(&cyclic));
    assert!(
        !edges
            .iter()
            .any(|(name, _)| ["source_state", "source_commit", "previous_sha256"].contains(name))
    );
}
