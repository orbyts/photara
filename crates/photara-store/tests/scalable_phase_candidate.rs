//! Unfrozen linked canonical phase proposal. Pure bytes, no qualified IO.
#[path = "scalable_operation_candidate/operations.rs"]
#[allow(
    dead_code,
    reason = "Reuse independent real-operation closure verifier"
)]
mod operations;
#[path = "scalable_wire_candidate/wire.rs"]
#[allow(
    dead_code,
    reason = "Reuse candidate frame parser without probe semantic dispatch"
)]
mod packed;
#[path = "scalable_phase_candidate/phases.rs"]
mod phases;
use phases::{Result, World, ensure};
use serde_json::{Value, json};
const OP_BYTES: &[u8] =
    include_bytes!("../../../docs/architecture/proposals/ps2/operations/linked-operations.json");
fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/phases/linked-phases.json"
    ))
    .unwrap()
}
fn ops() -> Value {
    serde_json::from_slice(OP_BYTES).unwrap()
}
fn ordered(world: &World<'_>) -> Result<Vec<Value>> {
    world.corpus["phase_order"]
        .as_array()
        .ok_or("phase order")?
        .iter()
        .map(|r| {
            world.corpus["records"]
                .as_object()
                .ok_or("records")?
                .values()
                .find(|e| e["sha256"] == r["sha256"])
                .map(|e| e["input"].clone())
                .ok_or("phase record")
        })
        .collect()
}
#[test]
fn canonical_phase_bytes_and_independent_remapped_closures() -> Result<()> {
    let corpus_value = corpus();
    let operations_value = ops();
    let world = World::new(&corpus_value, &operations_value, OP_BYTES)?;
    for n in ["old", "published", "released", "unlinked", "credited"] {
        world.snapshot(n, &corpus_value["snapshots"][n])?;
    }
    world.chain(&ordered(&world)?)
}
#[test]
fn original_reserve_control_width_and_consumed_cleanup_bounds_refuse() -> Result<()> {
    let corpus_value = corpus();
    let operations_value = ops();
    let world = World::new(&corpus_value, &operations_value, OP_BYTES)?;
    for (field, value) in [
        ("reserve", "1"),
        ("cleanup_bound", "0"),
        ("maximum_control_frame", "100"),
        ("codec", "unknown-v9"),
    ] {
        let mut a = world.named("admission")?;
        a[field] = json!(value);
        ensure(world.admission(&a).is_err(), "malformed original admitted")?;
    }
    for (index, field, value) in [
        (4, "consumed", "1048577"),
        (4, "held", "0"),
        (5, "consumed", "0"),
        (11, "project_credit", "999999"),
        (11, "filesystem_credit", "1"),
    ] {
        let mut p = ordered(&world)?;
        p[index][field] = json!(value);
        ensure(
            world.chain(&p).is_err(),
            "invalid phase accounting accepted",
        )?;
    }
    Ok(())
}
#[test]
fn original_codec_dispatch_preserves_bytes_across_reader_generation() -> Result<()> {
    let corpus_value = corpus();
    let operations_value = ops();
    World::new(&corpus_value, &operations_value, OP_BYTES)?;
    let entry = &corpus_value["records"]["admission"];
    let b = entry["canonical"].as_str().unwrap().as_bytes();
    let h = entry["sha256"].as_str().unwrap();
    ensure(
        World::dispatch(1, "example.ps2.phase-canonical-v1", b, h)?
            == World::dispatch(2, "example.ps2.phase-canonical-v1", b, h)?,
        "old original dispatch changed",
    )?;
    let pretty = serde_json::to_vec_pretty(&entry["input"]).unwrap();
    ensure(
        World::dispatch(2, "example.ps2.phase-canonical-v1", &pretty, h).is_err(),
        "reserialization replaced original proof",
    )?;
    ensure(
        World::dispatch(2, "example.ps2.phase-canonical-v2", b, h).is_err()
            && World::dispatch(3, "example.ps2.phase-canonical-v1", b, h).is_err(),
        "unsupported transition accepted",
    )
}
fn encoded(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
fn reference(v: &Value) -> Value {
    let b = encoded(v);
    json!({"kind":"json","sha256":packed::hash(&b),"byte_length":b.len().to_string()})
}
fn replace(corpus_value: &mut Value, name: &str, v: &Value) -> Value {
    let b = encoded(v);
    let r = reference(v);
    corpus_value["records"][name] = json!({"input":v,"canonical":String::from_utf8(b.clone()).unwrap(),"sha256":packed::hash(&b),"byte_length":b.len()});
    r
}
fn rehead(corpus_value: &mut Value, name: &str, root: Value) {
    let rr = replace(corpus_value, &format!("{name}-root"), &root);
    corpus_value["snapshots"][name]["root"] = rr;
    let b = &mut corpus_value["snapshots"][name]["bootstrap"];
    let mut commit: Value = serde_json::from_str(b["commit"].as_str().unwrap()).unwrap();
    commit["root_set"] = root;
    let bytes = encoded(&commit);
    b["commit"] = json!(String::from_utf8(bytes.clone()).unwrap());
    let mut head: Value = serde_json::from_str(b["head"].as_str().unwrap()).unwrap();
    head["commit_sha256"] = json!(packed::hash(&bytes));
    b["head"] = json!(String::from_utf8(encoded(&head)).unwrap());
}
#[test]
fn coherent_undercharge_and_extra_owner_are_rejected_before_admission() -> Result<()> {
    let mut corpus_value = corpus();
    let operations_value = ops();
    let world = World::new(&corpus_value, &operations_value, OP_BYTES)?;
    ensure(
        world
            .snapshot("extra-owner", &corpus_value["snapshots"]["extra-owner"])
            .is_err(),
        "coherent unrelated owner claim accepted",
    )?;
    let mut a = world.named("published-accounting")?;
    let prior = a["tips"][0]["registered_charge"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    a["tips"][0]["registered_charge"] = json!("0");
    a["total_charge"] = json!((phases::decimal(&a["total_charge"])? - prior).to_string());
    let old = corpus_value["records"]["published-accounting"]["sha256"].clone();
    let ar = replace(&mut corpus_value, "published-accounting", &a);
    let mut union = corpus_value["records"]["published-union"]["input"].clone();
    for r in union["entries"].as_array_mut().unwrap() {
        if r["sha256"] == old {
            *r = ar.clone();
        }
    }
    union["entries"]
        .as_array_mut()
        .unwrap()
        .sort_by(|a, b| a["sha256"].as_str().cmp(&b["sha256"].as_str()));
    let ur = replace(&mut corpus_value, "published-union", &union);
    let mut root = corpus_value["records"]["published-root"]["input"].clone();
    root["inventory"] = ur;
    root["placement"]["accounting"] = ar;
    rehead(&mut corpus_value, "published", root);
    let world = World::new(&corpus_value, &operations_value, OP_BYTES)?;
    ensure(
        world
            .snapshot("published", &corpus_value["snapshots"]["published"])
            .is_err(),
        "rehashed ancestors authenticated undercharge",
    )
}
#[test]
fn independent_roles_ignore_other_locator_but_cover_own_implementation() -> Result<()> {
    let corpus_value = corpus();
    let operations_value = ops();
    let world = World::new(&corpus_value, &operations_value, OP_BYTES)?;
    for (good, bad) in [("active", "recovery"), ("recovery", "active")] {
        let mut s = corpus_value["snapshots"]["published"].clone();
        let root = world.bootstrap(&s, 3)?;
        let p = &root["placement"][format!("{bad}_locator")];
        let id = p["allocation_id"].as_str().unwrap();
        let offset = usize::try_from(phases::decimal(&p["offset"])?).map_err(|_| "offset")?;
        let mut bytes = packed::unhex(s["files"][id]["hex"].as_str().unwrap())?;
        bytes[offset + 16] = b'!';
        s["files"][id]["hex"] = json!(hex(&bytes));
        world.role("published", &s, good)?;
        ensure(
            world.role("published", &s, bad).is_err(),
            "corrupt selected locator accepted",
        )?;
    }
    let mut s = corpus_value["snapshots"]["published"].clone();
    let source = world.named("retirement-admission")?["source"]["witness"]["allocation_id"]
        .as_str()
        .unwrap()
        .to_owned();
    s["files"].as_object_mut().unwrap().remove(&source);
    world.role("published", &s, "active")?;
    ensure(
        world.role("published", &s, "recovery").is_err(),
        "recovery borrowed active physical closure",
    )
}
fn hex(bytes: &[u8]) -> String {
    let alphabet = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(char::from(alphabet[usize::from(byte >> 4)]));
        result.push(char::from(alphabet[usize::from(byte & 15)]));
    }
    result
}
#[test]
fn optional_extension_object_shape_does_not_add_a_retention_edge() -> Result<()> {
    let mut corpus_value = corpus();
    let operations_value = ops();
    let mut root = corpus_value["records"]["published-root"]["input"].clone();
    root["extensions"]["example.nonedge"] =
        json!({"kind":"json","sha256":"0".repeat(64),"byte_length":"999"});
    rehead(&mut corpus_value, "published", root);
    let world = World::new(&corpus_value, &operations_value, OP_BYTES)?;
    world.snapshot("published", &corpus_value["snapshots"]["published"])?;
    Ok(())
}
#[test]
fn all_twelve_pin_roles_fence_retirement_and_source_reappearance_fences_credit() -> Result<()> {
    let corpus_value = corpus();
    let operations_value = ops();
    let world = World::new(&corpus_value, &operations_value, OP_BYTES)?;
    ensure(
        world
            .eligible(&corpus_value["snapshots"]["old"], &[])
            .is_err(),
        "current active source retained",
    )?;
    ensure(
        world
            .eligible(&corpus_value["snapshots"]["published"], &[])
            .is_err(),
        "current recovery source retained",
    )?;
    for class in [
        "AcceptedJournal",
        "Undo",
        "History",
        "ConversionSource",
        "Export",
        "Backup",
        "ReaderLease",
        "UnresolvedIntent",
        "Relocation",
        "ResourceObligation",
    ] {
        ensure(
            world
                .eligible(
                    &corpus_value["snapshots"]["released"],
                    &[(class.into(), corpus_value["snapshots"]["published"].clone())],
                )
                .is_err(),
            "extra pin source retained",
        )?;
    }
    world.eligible(&corpus_value["snapshots"]["released"], &[])?;
    let source = world.named("retirement-admission")?["source"]["witness"]["allocation_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut reappeared = corpus_value["snapshots"]["credited"].clone();
    reappeared["files"][&source] = corpus_value["snapshots"]["released"]["files"][&source].clone();
    ensure(
        world.source_absent(&reappeared).is_err(),
        "reappeared original bytes accepted as absence",
    )?;
    let completed = world.named("credited")?;
    for field in ["original", "authorization_sha256", "project_credit"] {
        let mut forged = completed.clone();
        forged[field] = Value::Null;
        ensure(
            world.credit_retry(&corpus_value["snapshots"]["credited"], &forged)
                == Err("exact original terminal record"),
            "forged completion accepted",
        )?;
    }
    ensure(
        world.credit_retry(&corpus_value["snapshots"]["credited"], &completed)? == 0,
        "duplicate project credit",
    )?;
    ensure(
        world.credit_retry(&reappeared, &completed).is_err(),
        "reappeared source retry accepted",
    )
}
#[test]
fn source_destination_substitution_mixed_modes_and_unreachable_suffix_refuse() -> Result<()> {
    let corpus_value = corpus();
    let operations_value = ops();
    let world = World::new(&corpus_value, &operations_value, OP_BYTES)?;
    let source = world.named("retirement-admission")?["source"]["witness"]["allocation_id"]
        .as_str()
        .unwrap()
        .to_owned();
    for id in [
        source,
        corpus_value["records"]["compact"]["input"]["data"]["witness"]["allocation_id"]
            .as_str()
            .unwrap()
            .to_owned(),
    ] {
        let mut s = corpus_value["snapshots"]["published"].clone();
        s["files"][&id]["witness"]["inode"] = json!("9999");
        ensure(
            world.snapshot("published", &s).is_err(),
            "same-byte inode replacement adopted",
        )?;
    }
    let data = packed::unhex(corpus_value["complete_data_hex"].as_str().unwrap())?;
    let meta = packed::unhex(corpus_value["complete_metadata_hex"].as_str().unwrap())?;
    for field in [
        "original_sha256",
        "framed_sha256",
        "frame_count",
        "final_end",
    ] {
        let mut p = world.named("compact")?;
        p["data"][field] = if field.contains("sha256") {
            json!("0".repeat(64))
        } else {
            json!("1")
        };
        ensure(
            world.compact(&p, &data, &meta).is_err(),
            "altered original suffix commitment accepted",
        )?;
    }
    let mut mixed = world.named("compact")?;
    mixed["full_data"] = json!({});
    ensure(
        world.compact(&mixed, &data, &meta).is_err(),
        "mixed recipe modes",
    )?;
    let mut full = world.named("compact")?;
    full["mode"] = json!("full");
    for (manifest, full_name, bytes, prefix) in [
        ("data", "full_data", &data, "original_data_prefix_hex"),
        (
            "metadata",
            "full_metadata",
            &meta,
            "original_metadata_prefix_hex",
        ),
    ] {
        let start = packed::unhex(corpus_value[prefix].as_str().unwrap())?.len();
        full[full_name] =
            json!({"manifest":full[manifest].clone(),"framed_hex":hex(&bytes[start..])});
        full[manifest] = Value::Null;
    }
    world.compact(&full, &data, &meta)?;
    let mut damaged = data.clone();
    damaged[usize::try_from(corpus_value["unreachable_frame_offset"].as_u64().unwrap()).unwrap()
        + 16] = b'!';
    let mut physical = corpus_value["snapshots"]["released"].clone();
    let id = world.named("compact")?["data"]["witness"]["allocation_id"]
        .as_str()
        .unwrap()
        .to_owned();
    physical["files"][&id]["hex"] = json!(hex(&damaged));
    world.snapshot("released", &physical)?;
    ensure(
        world
            .compact(&world.named("compact")?, &damaged, &meta)
            .is_err(),
        "unreachable raw frame corruption was omitted",
    )?;
    let mut tail = data;
    tail.push(0);
    ensure(
        world
            .compact(&world.named("compact")?, &tail, &meta)
            .is_err(),
        "unknown extra tail",
    )
}

#[test]
fn birth_binding_precedes_truncate_and_only_exact_promotion_pair_is_allowed() -> Result<()> {
    let corpus_value = corpus();
    let operations_value = ops();
    let world = World::new(&corpus_value, &operations_value, OP_BYTES)?;
    world.birth(&world.named("birth-proof")?)?;
    for mode in 0..4 {
        let mut proof = world.named("birth-proof")?;
        match mode {
            0 => proof["events"].as_array_mut().unwrap().swap(2, 3),
            1 => proof["promotion_pair"]["final"]["witness"]["inode"] = json!("9999"),
            2 => proof["promotion_pair"]["stage"]["links"] = json!("3"),
            _ => proof["stage_absent"] = json!(false),
        }
        ensure(
            world.birth(&proof).is_err(),
            "unknown/unbound birth occupancy accepted",
        )?;
    }
    Ok(())
}

#[test]
fn post_unlink_proof_has_no_historical_source_fallback() -> Result<()> {
    let mut c = corpus();
    let source =
        c["records"]["retirement-admission"]["input"]["source"]["witness"]["allocation_id"]
            .as_str()
            .unwrap()
            .to_owned();
    for s in c["snapshots"].as_object_mut().unwrap().values_mut() {
        s["files"].as_object_mut().unwrap().remove(&source);
    }
    c.as_object_mut().unwrap().remove("original_source_hex");
    let o = ops();
    let w = World::new(&c, &o, OP_BYTES)?;
    w.compact(
        &w.named("compact")?,
        &packed::unhex(c["complete_data_hex"].as_str().unwrap())?,
        &packed::unhex(c["complete_metadata_hex"].as_str().unwrap())?,
    )?;
    ensure(
        w.credit_retry(&c["snapshots"]["credited"], &w.named("credited")?)? == 0,
        "source-free repeat credit",
    )?;
    Ok(())
}

#[test]
fn coherently_rehashed_outer_bootstrap_mismatch_refuses() -> Result<()> {
    let mut c = corpus();
    let mut commit: Value = serde_json::from_str(
        c["snapshots"]["old"]["bootstrap"]["commit"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    commit["bootstrap_sha256"] = json!("0".repeat(64));
    let bytes = encoded(&commit);
    let mut head: Value =
        serde_json::from_str(c["snapshots"]["old"]["bootstrap"]["head"].as_str().unwrap()).unwrap();
    head["commit_sha256"] = json!(packed::hash(&bytes));
    c["snapshots"]["old"]["bootstrap"]["commit"] = json!(String::from_utf8(bytes).unwrap());
    c["snapshots"]["old"]["bootstrap"]["head"] = json!(String::from_utf8(encoded(&head)).unwrap());
    let o = ops();
    let w = World::new(&c, &o, OP_BYTES)?;
    ensure(
        w.snapshot("old", &c["snapshots"]["old"]).is_err(),
        "outer bootstrap binding ignored",
    )
    .unwrap();
    Ok(())
}

#[test]
fn supplied_snapshot_suffix_and_supplied_phase_bounds_refuse() -> Result<()> {
    let original = corpus();
    let o = ops();
    for name in ["released", "unlinked", "credited"] {
        let mut c = original.clone();
        let id = c["records"]["compact"]["input"]["data"]["witness"]["allocation_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut bytes = packed::unhex(c["snapshots"][name]["files"][&id]["hex"].as_str().unwrap())?;
        let offset = usize::try_from(c["unreachable_frame_offset"].as_u64().unwrap()).unwrap() + 20;
        bytes[offset] ^= 1;
        c["snapshots"][name]["files"][&id]["hex"] = json!(hex(&bytes));
        let w = World::new(&c, &o, OP_BYTES)?;
        ensure(
            w.compact_snapshot(&c["snapshots"][name]).is_err(),
            "actual snapshot suffix ignored",
        )?;
        ensure(
            w.chain(&ordered(&w)?).is_err(),
            "chain used detached suffix",
        )?;
        if name == "credited" {
            ensure(
                w.credit_retry(&c["snapshots"][name], &w.named("credited")?)
                    .is_err(),
                "retry ignored suffix",
            )?;
        }
    }
    let w = World::new(&original, &o, OP_BYTES)?;
    let mut p = ordered(&w)?;
    p.last_mut().unwrap()["extensions"] = json!({"example.large":"x".repeat(17000)});
    ensure(w.chain(&p).is_err(), "actual control cap ignored")?;
    for index in [0, 1] {
        let mut p = ordered(&w)?;
        p[index]["births"] =
            json!([original["records"]["admission"]["input"]["source_witnesses"][0]]);
        ensure(
            w.chain(&p).is_err(),
            "unrelated registered generation accepted as birth",
        )?;
    }
    Ok(())
}

#[test]
fn coherent_standing_ticket_and_commit_transition_tampering_refuses() -> Result<()> {
    let o = ops();
    for mode in ["standing", "ticket-charge", "duplicate-ticket"] {
        let mut c = corpus();
        let name = if mode == "standing" {
            "published"
        } else {
            "released"
        };
        let key = format!("{name}-accounting");
        let mut a = c["records"][&key]["input"].clone();
        if mode == "standing" {
            a["total_charge"] = json!(
                (phases::decimal(&a["total_charge"])? - phases::decimal(&a["standing_control"])?)
                    .to_string()
            );
            a["standing_control"] = json!("0");
        } else if mode == "ticket-charge" {
            a["tickets"][0]["registered_charge"] = json!("0");
        } else {
            let t = a["tickets"][0].clone();
            a["tickets"].as_array_mut().unwrap().push(t);
        }
        let prior = c["records"][&key]["sha256"].clone();
        let ar = replace(&mut c, &key, &a);
        let union_key = format!("{name}-union");
        let mut union = c["records"][&union_key]["input"].clone();
        for r in union["entries"].as_array_mut().unwrap() {
            if r["sha256"] == prior {
                *r = ar.clone();
            }
        }
        union["entries"]
            .as_array_mut()
            .unwrap()
            .sort_by(|a, b| a["sha256"].as_str().cmp(&b["sha256"].as_str()));
        let ur = replace(&mut c, &union_key, &union);
        let mut root = c["records"][format!("{name}-root")]["input"].clone();
        root["placement"]["accounting"] = ar;
        root["inventory"] = ur;
        rehead(&mut c, name, root);
        let w = World::new(&c, &o, OP_BYTES)?;
        ensure(
            w.snapshot(name, &c["snapshots"][name]).is_err(),
            "coherent accounting mutation accepted",
        )?;
    }
    for mode in ["revision", "parent", "nil-write"] {
        let mut c = corpus();
        let mut commit: Value = serde_json::from_str(
            c["snapshots"]["published"]["bootstrap"]["commit"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        if mode == "revision" {
            commit["package_revision"] = json!("1");
        } else if mode == "nil-write" {
            commit["write_id"] = json!("00000000-0000-0000-0000-000000000000");
        } else {
            commit["parent"]["sha256"] = json!("0".repeat(64));
        }
        let mut head: Value = serde_json::from_str(
            c["snapshots"]["published"]["bootstrap"]["head"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        head["commit_sha256"] = json!(packed::hash(&encoded(&commit)));
        c["snapshots"]["published"]["bootstrap"]["commit"] =
            json!(String::from_utf8(encoded(&commit)).unwrap());
        c["snapshots"]["published"]["bootstrap"]["head"] =
            json!(String::from_utf8(encoded(&head)).unwrap());
        let w = World::new(&c, &o, OP_BYTES)?;
        ensure(
            w.transitions().is_err(),
            "coherent transition mutation accepted",
        )?;
    }
    Ok(())
}
