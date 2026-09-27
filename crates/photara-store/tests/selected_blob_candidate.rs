//! Preparatory same-project D19 bytes and adapters. NOT yet shared HEAD validation.
#[allow(
    dead_code,
    reason = "Exercise the generic accounting subproof without claiming a HEAD reader"
)]
#[path = "integrated_wire_candidate/accounting.rs"]
mod accounting;
#[path = "selected_blob_candidate/legacy.rs"]
mod legacy;
#[path = "selected_blob_candidate/media.rs"]
mod media;
use legacy::{Result, TrustedLegacy, ensure};
use media::{Description, Media};
use photara_store::package::{ObjectKind, ObjectRef};
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn corpus() -> Value {
    let bytes =
        include_bytes!("../../../docs/architecture/proposals/ps2/selected-blob/linked.json");
    assert!(bytes.len() <= 8 * 1024 * 1024);
    serde_json::from_slice(bytes).unwrap()
}
fn json_ref(v: &Value) -> ObjectRef {
    serde_json::from_value(v.clone()).unwrap()
}
/// Extract supplied framed metadata for adapter testing. This does not prove
/// selected locators, ownership or the containing HEAD's exact global closure.
fn framed(c: &Value) -> Result<BTreeMap<ObjectRef, Vec<u8>>> {
    let mut records = BTreeMap::new();
    let mut aggregate = 0usize;
    for allocation in c["allocations"].as_object().ok_or("allocations")?.values() {
        if allocation["layout"] != "framed-json" {
            continue;
        }
        let bytes = media::unhex(allocation["hex"].as_str().ok_or("framed bytes")?)?;
        aggregate = aggregate
            .checked_add(bytes.len())
            .ok_or("framed overflow")?;
        ensure(aggregate <= 16 * 1024 * 1024, "framed aggregate bound")?;
        let mut at = 0;
        while at < bytes.len() {
            let header = bytes.get(at..at + 16).ok_or("frame header")?;
            ensure(
                &header[..8] == b"PS2PKD01" && header[9..12] == [0, 0, 0],
                "frame magic/reserved",
            )?;
            let count =
                u32::from_le_bytes(header[12..16].try_into().map_err(|_| "frame length")?) as usize;
            let end = at
                .checked_add(16)
                .and_then(|n| n.checked_add(count))
                .ok_or("frame overflow")?;
            let body = bytes.get(at + 16..end).ok_or("frame body")?;
            match header[8] {
                0 => ensure(body.iter().all(|b| *b == 0), "padding zeros")?,
                1..=3 => {
                    legacy::parse(body)?;
                    let r = legacy::reference(body)?;
                    if let Some(old) = records.insert(r, body.to_vec()) {
                        ensure(old == body, "duplicate canonical metadata")?;
                    }
                }
                _ => return Err("frame tag"),
            }
            at = end;
        }
    }
    Ok(records)
}
fn registration(c: &Value) -> (ObjectRef, String, Description) {
    let r = json_ref(&c["expected"]["blob"]);
    let id = c["expected"]["blob_allocation"]
        .as_str()
        .unwrap()
        .to_owned();
    let e = &c["original_evidence"]["allocations"][&id];
    (
        r,
        id,
        Description {
            extent: e["extent"].as_str().unwrap().parse().unwrap(),
            device: e["witness"]["device"].as_str().unwrap().into(),
            inode: e["witness"]["inode"].as_str().unwrap().into(),
        },
    )
}
fn structural_adapter(
    trusted: &TrustedLegacy,
    records: &BTreeMap<ObjectRef, Vec<u8>>,
    media: &Media,
    r: &ObjectRef,
    id: &str,
    registered: &Description,
) -> Result<()> {
    let selected = trusted.verify_semantic_metadata(
        &trusted.authored,
        &trusted.history,
        |reference| {
            records
                .get(reference)
                .cloned()
                .ok_or("missing current metadata")
        },
        |reference| {
            ensure(reference == r, "exact selected Blob mapping")?;
            media.structural(reference, id, registered)
        },
    )?;
    ensure(selected == trusted.closure, "original selected closure")
}
#[test]
fn preparatory_canonical_corpus_preserves_actual_d19_project_and_typed_legacy_closure() {
    let c = corpus();
    let trusted = TrustedLegacy::d19().unwrap();
    assert_eq!(trusted.project, "62000000-0000-4000-8000-000000020000");
    assert_eq!(trusted.library, "62000000-0000-4000-8000-000000020001");
    assert_eq!(trusted.authored_revision, "1");
    assert_eq!(trusted.setup_verified_blobs, 1);
    assert_eq!(
        c["original_package_sha256"],
        trusted.original_package_sha256
    );
    assert_eq!(
        c["bootstrap"]["manifest"].as_str().unwrap().as_bytes(),
        trusted.bootstrap_bytes
    );
    for record in c["records"].as_object().unwrap().values() {
        let bytes = record["canonical"].as_str().unwrap().as_bytes();
        let value = legacy::parse(bytes).unwrap();
        assert_eq!(photara_core::canonical_json(&value).unwrap(), bytes);
        assert_eq!(record["sha256"], legacy::hash(bytes));
        assert_eq!(record["byte_length"], bytes.len());
    }
    let head: Value = serde_json::from_str(c["bootstrap"]["head"].as_str().unwrap()).unwrap();
    let commit_bytes = c["bootstrap"]["commit"].as_str().unwrap().as_bytes();
    assert_eq!(head["commit_sha256"], legacy::hash(commit_bytes));
    let commit = legacy::parse(commit_bytes).unwrap();
    assert_eq!(commit["project_id"], trusted.project);
    assert_eq!(commit["root_set"]["library_id"], trusted.library);
    assert!(commit["root_set"]["conversion_source"].is_null());
    assert_eq!(commit["root_set"]["active"], commit["root_set"]["recovery"]);
    let records = framed(&c).unwrap();
    let state = legacy::parse(&records[&json_ref(&commit["root_set"]["active"])]).unwrap();
    assert!(state["resource_state"].is_null() && state["journal_inclusion"].is_null());
    assert_eq!(state["accepted"]["through_ordinal"], "0");
    assert_eq!(json_ref(&state["authored"]), trusted.authored);
    assert_eq!(json_ref(&state["history"]), trusted.history);
    let media = Media::load(&c).unwrap();
    let (r, id, e) = registration(&c);
    structural_adapter(&trusted, &records, &media, &r, &id, &e).unwrap();
    assert_eq!(media.counters(), (0, 0));
    assert!(
        trusted
            .closure
            .iter()
            .next()
            .is_some_and(|r| r.kind == ObjectKind::Blob)
    );
}
#[test]
fn metadata_only_adapter_reads_no_current_media_and_explicit_audit_detects_corruption() {
    let trusted = TrustedLegacy::d19().unwrap(); // Strong reference setup is deliberately outside instrumentation.
    let c = corpus();
    let records = framed(&c).unwrap();
    let (r, id, e) = registration(&c);
    let media = Media::load(&c).unwrap();
    structural_adapter(&trusted, &records, &media, &r, &id, &e).unwrap();
    assert_eq!(media.counters(), (0, 0));
    media.audit(&r, &id, &e).unwrap();
    assert_eq!(media.counters(), (5, 1));
    let mut corrupt = Media::load(&c).unwrap();
    corrupt.corrupt_byte(&id).unwrap();
    structural_adapter(&trusted, &records, &corrupt, &r, &id, &e).unwrap();
    assert_eq!(corrupt.counters(), (0, 0));
    assert_eq!(corrupt.audit(&r, &id, &e), Err("explicit raw audit digest"));
    assert_eq!(corrupt.counters(), (5, 1));
    let mut replaced = Media::load(&c).unwrap();
    replaced.replace_inode(&id).unwrap();
    assert_eq!(
        structural_adapter(&trusted, &records, &replaced, &r, &id, &e),
        Err("original raw extent/witness")
    );
    assert_eq!(replaced.counters(), (0, 0));
}
#[test]
fn preserved_snapshot_adapter_refuses_other_project_metadata_or_missing_selected_blob() {
    let trusted = TrustedLegacy::d19().unwrap();
    let c = corpus();
    let mut records = framed(&c).unwrap();
    let (r, id, e) = registration(&c);
    let media = Media::load(&c).unwrap();
    let mut authored = legacy::parse(&records[&trusted.authored]).unwrap();
    authored["project_id"] = json!("10000000-0000-4000-8000-000000000001");
    records.insert(
        trusted.authored.clone(),
        photara_core::canonical_json(&authored).unwrap(),
    );
    assert_eq!(
        structural_adapter(&trusted, &records, &media, &r, &id, &e),
        Err("D19 selected legacy bytes changed")
    );
    let records = framed(&c).unwrap();
    assert_eq!(
        structural_adapter(&trusted, &records, &media, &r, "missing-raw-allocation", &e),
        Err("missing raw allocation")
    );
    assert_eq!(media.counters(), (0, 0));
}

#[test]
fn preparatory_generic_ledger_counts_raw_allocation_without_reading_its_bytes() {
    let c = corpus();
    let trusted = TrustedLegacy::d19().unwrap();
    let media = Media::load(&c).unwrap();
    let records = framed(&c).unwrap();
    let loose: Vec<Value> = c["loose"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| legacy::parse(v.as_str().unwrap().as_bytes()).unwrap())
        .collect();
    let ledger = loose
        .iter()
        .find(|v| v["schema"]["id"] == "photara.storage.ledger")
        .unwrap();
    let hold = records
        .values()
        .map(|b| legacy::parse(b).unwrap())
        .find(|v| v["schema"]["id"] == "photara.storage.hold-leaf")
        .unwrap();
    let evidence = accounting_evidence(&c, &media, &trusted.project);
    let resolve = |r: &Value| -> accounting::Result<Value> {
        let r: ObjectRef = serde_json::from_value(r.clone()).map_err(|_| "accounting reference")?;
        legacy::parse(
            records
                .get(&r)
                .ok_or("missing supplied accounting metadata")?,
        )
    };
    let proof = accounting::verify(ledger, &hold, resolve, &evidence).unwrap();
    assert_eq!(
        (
            proof.tips,
            proof.sealed,
            proof.retained,
            proof.standing,
            proof.directory
        ),
        (262_144, 4096, 0, 65_536, 16_384)
    );
    assert_eq!(proof.total, 348_160);
    assert_eq!(proof.allocations.len(), 3);
    assert_eq!(media.counters(), (0, 0));
    let mut under = ledger.clone();
    under["total_charge"] = json!("344064");
    assert!(accounting::verify(&under, &hold, resolve, &evidence).is_err());
    let mut namespace = ledger.clone();
    namespace["directory_allowance"] = json!("0");
    namespace["total_charge"] = json!("331776");
    assert!(accounting::verify(&namespace, &hold, resolve, &evidence).is_err());
    assert_eq!(media.counters(), (0, 0));
}

fn accounting_evidence(c: &Value, media: &Media, project: &str) -> accounting::Evidence {
    let original = &c["original_evidence"];
    let mut evidence = accounting::Evidence {
        project_id: project.into(),
        profile: original["profile"].as_str().unwrap().into(),
        incarnation: original["incarnation"].as_str().unwrap().into(),
        allocations: BTreeMap::new(),
        retained_files: BTreeMap::new(),
        conversion: None,
        standing_control: original["standing_control"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
        directory_allowance: original["directory_allowance"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
        retained_directory_allowance: 0,
    };
    for (id, r) in original["allocations"].as_object().unwrap() {
        let is_raw = r["layout"] == "whole-blob";
        let (extent, device, inode) = if is_raw {
            let d = media.describe(id).unwrap();
            (
                d.extent,
                d.device.parse().unwrap(),
                d.inode.parse().unwrap(),
            )
        } else {
            let a = &c["allocations"][id];
            (
                media::unhex(a["hex"].as_str().unwrap()).unwrap().len() as u64,
                a["witness"]["device"].as_str().unwrap().parse().unwrap(),
                a["witness"]["inode"].as_str().unwrap().parse().unwrap(),
            )
        };
        assert_eq!(
            extent,
            r["extent"].as_str().unwrap().parse::<u64>().unwrap()
        );
        assert_eq!(
            device,
            r["witness"]["device"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        );
        assert_eq!(
            inode,
            r["witness"]["inode"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        );
        evidence.allocations.insert(
            id.clone(),
            accounting::AllocationEvidence {
                arena: r["arena"].as_str().unwrap().into(),
                subject_kind: if is_raw { "whole-blob" } else { "pack" }.into(),
                extent,
                registered_charge: r["registered_charge"].as_str().unwrap().parse().unwrap(),
                device,
                inode,
            },
        );
    }
    evidence
}
