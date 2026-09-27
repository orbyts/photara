//! New epoch admission-only proposal. No native effects or writable qualification.
#![allow(dead_code)]
#[path = "integrated_wire_candidate/accounting.rs"]
mod accounting;
#[path = "selected_admission_candidate/prepare.rs"]
mod prepare;
#[path = "integrated_wire_candidate/resources.rs"]
mod resources;
#[path = "integrated_wire_candidate/wire.rs"]
mod wire;

#[path = "selected_admission_candidate/admission.rs"]
mod admission;

fn corpus() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/admission/linked.json"
    ))
    .unwrap()
}
fn original() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/integrated/linked.json"
    ))
    .unwrap()
}
#[test]
fn admission_full_dry_run_and_actual_successor() -> wire::Result<()> {
    let c = corpus();
    let base = original();
    let old = wire::World::load(&base)?;
    let oldproof = wire::verify(&old)?;
    let prepared = prepare::verify(&oldproof)?;
    let future = wire::World::load_transition(&c["prospective"])?;
    let futureproof = wire::verify_transition(&future, &old, &prepared.authored)?;
    let decoded = decoder::regenerate(&old, &oldproof, &prepared)?;
    for (id, bytes) in &decoded.files {
        wire::ensure(
            future.allocations[id].bytes == *bytes,
            "independent exact regenerated append bytes",
        )?;
    }
    wire::ensure(
        decoded.target == c["dry_run"]["target"]
            && decoded.base_inventory == c["dry_run"]["base_inventory"]
            && decoded.root_placements == c["dry_run"]["root_placements"],
        "independent regenerated complete target",
    )?;
    let next = admission::load_selected(&c, &base)?;
    let envelope = next.loose(&next.commit["root_set"]["placement"]["accounting"])?;
    let ledger = next.loose(&envelope["ledger"])?;
    let hold = next.loose(&envelope["holds"])?;
    assert!(matches!(
        accounting::metadata(&ledger, &hold, |_| Err("unexpected transition traversal")),
        Err("accounting nonempty holds unsupported in settled reader")
    ));
    let p = admission::verify(&c, &old, &next, &future, &oldproof, &prepared, &futureproof)?;
    println!(
        "admission reserve={} future_growth={} peak={} global={}",
        p.reserved,
        p.worst_growth,
        p.control_peak,
        p.global.len()
    );
    Ok(())
}

#[path = "selected_admission_candidate/decoder.rs"]
mod decoder;

fn check(c: &serde_json::Value) -> wire::Result<admission::Proof> {
    let base = original();
    let old = wire::World::load(&base)?;
    let proof = wire::verify(&old)?;
    let prepared = prepare::verify(&proof)?;
    let future = wire::World::load_transition(&c["prospective"])?;
    let futureproof = wire::verify_transition(&future, &old, &prepared.authored)?;
    let next = admission::load_selected(c, &base)?;
    admission::verify(c, &old, &next, &future, &proof, &prepared, &futureproof)
}
fn rebind(
    c: &mut serde_json::Value,
    change: impl FnOnce(&mut serde_json::Value, &mut serde_json::Value),
) {
    use serde_json::{Value, json};
    let selected = &mut c["selected"];
    let mut values = selected["loose"]
        .as_object()
        .unwrap()
        .values()
        .map(|s| serde_json::from_str::<Value>(s.as_str().unwrap()).unwrap())
        .collect::<Vec<_>>();
    let find = |name: &str| {
        values
            .iter()
            .find(|v| v["schema"]["id"] == name)
            .unwrap()
            .clone()
    };
    let mut original = find("photara.storage.original-admission");
    let mut phase = find("photara.storage.operation-phase");
    let mut hold = find("photara.storage.hold-leaf");
    let mut envelope = find("photara.storage.accounting-envelope");
    let mut overlay = find("photara.package.inventory-overlay");
    change(&mut original, &mut phase);
    phase["original"] = wire::reference(&original);
    hold["entries"][0]["original"] = wire::reference(&original);
    hold["entries"][0]["phase"] = wire::reference(&phase);
    hold["reserved"] = original["reserve"].clone();
    hold["remaining"] = phase["remaining"].clone();
    envelope["holds"] = wire::reference(&hold);
    values.retain(|v| {
        ![
            "photara.storage.original-admission",
            "photara.storage.operation-phase",
            "photara.storage.hold-leaf",
            "photara.storage.accounting-envelope",
            "photara.package.inventory-overlay",
        ]
        .contains(&v["schema"]["id"].as_str().unwrap_or(""))
    });
    values.extend([original, phase, hold, envelope.clone()]);
    let mut refs = values.iter().map(wire::reference).collect::<Vec<_>>();
    refs.sort_by_key(|v| wire::key(v).unwrap());
    overlay["controls"] = json!(refs);
    values.push(overlay.clone());
    selected["loose"] = json!(
        values
            .iter()
            .map(|v| (
                wire::hash(&wire::encode(v)),
                String::from_utf8(wire::encode(v)).unwrap()
            ))
            .collect::<std::collections::BTreeMap<_, _>>()
    );
    let mut commit: Value =
        serde_json::from_str(selected["bootstrap"]["commit"].as_str().unwrap()).unwrap();
    commit["root_set"]["inventory"] = wire::reference(&overlay);
    commit["root_set"]["placement"]["accounting"] = wire::reference(&envelope);
    let mut head: Value =
        serde_json::from_str(selected["bootstrap"]["head"].as_str().unwrap()).unwrap();
    head["commit_sha256"] = json!(wire::hash(&wire::encode(&commit)));
    selected["bootstrap"]["commit"] = json!(String::from_utf8(wire::encode(&commit)).unwrap());
    selected["bootstrap"]["head"] = json!(String::from_utf8(wire::encode(&head)).unwrap());
}
#[test]
fn coherent_admission_original_phase_and_bound_swaps_refuse() {
    use serde_json::json;
    for mode in [
        "underreserve",
        "capacity",
        "controls",
        "phase",
        "old-ledger",
        "scope",
        "target",
        "retention",
        "codec",
        "old-hold",
        "unknown",
    ] {
        let mut c = corpus();
        rebind(&mut c, |o, p| match mode {
            "underreserve" => {
                o["reserve"] = json!("65536");
                p["remaining"] = json!("65536");
            }
            "capacity" => o["project_limit"] = json!("1"),
            "controls" => o["control_bounds"]["aggregate_control_bytes"] = json!("65536"),
            "phase" => p["stage"] = json!("published"),
            "old-ledger" => o["old"]["ledger"]["total_charge"] = json!("0"),
            "scope" => o["scope"]["incarnation"] = json!("96000000-0000-4000-8000-000000000002"),
            "target" => {
                o["semantic_target"]["active"] = o["old"]["commit"]["root_set"]["active"].clone();
            }
            "retention" => o["retention_intent"] = o["request"]["intent"].clone(),
            "codec" => o["payload_recipe"]["codec"] = json!("photara.codec.unsupported-v2"),
            "old-hold" => o["old_hold"] = o["request"]["receipt"].clone(),
            _ => {
                o["unknown"] = json!(true);
            }
        });
        let before = wire::hash(&wire::encode(&c));
        let result = check(&c);
        assert!(result.is_err(), "coherent {mode} accepted");
        assert_eq!(
            before,
            wire::hash(&wire::encode(&c)),
            "refusal changed input"
        );
    }
}
#[test]
fn actual_successor_head_and_extra_control_refuse() {
    use serde_json::{Value, json};
    for mode in ["parent", "root", "extra"] {
        let mut c = corpus();
        let s = &mut c["selected"];
        let mut commit: Value =
            serde_json::from_str(s["bootstrap"]["commit"].as_str().unwrap()).unwrap();
        match mode {
            "parent" => commit["parent"]["commit_sha256"] = json!("0".repeat(64)),
            "root" => commit["root_set"]["placement"]["generation"] = json!("2"),
            _ => {
                let v = json!({"schema":{"id":"photara.storage.unknown-control","version":1},"project_id":wire::PROJECT,"extensions":{}});
                s["loose"][wire::hash(&wire::encode(&v))] =
                    json!(String::from_utf8(wire::encode(&v)).unwrap());
            }
        }
        let mut head: Value =
            serde_json::from_str(s["bootstrap"]["head"].as_str().unwrap()).unwrap();
        head["commit_sha256"] = json!(wire::hash(&wire::encode(&commit)));
        s["bootstrap"]["head"] = json!(String::from_utf8(wire::encode(&head)).unwrap());
        s["bootstrap"]["commit"] = json!(String::from_utf8(wire::encode(&commit)).unwrap());
        assert!(check(&c).is_err(), "actual {mode} accepted");
    }
}
#[test]
fn decoder_reconstructs_without_any_future_corpus() -> wire::Result<()> {
    let base = original();
    let old = wire::World::load(&base)?;
    let proof = wire::verify(&old)?;
    let prepared = prepare::verify(&proof)?;
    let first = decoder::regenerate(&old, &proof, &prepared)?;
    let second = decoder::regenerate(&old, &proof, &prepared)?;
    wire::ensure(
        first.files == second.files
            && first.target == second.target
            && first.root_placements == second.root_placements,
        "standalone decoder determinism",
    )
}
#[test]
fn same_byte_inode_or_original_source_substitution_refuses() -> wire::Result<()> {
    use serde_json::json;
    let c = corpus();
    let base = original();
    let old = wire::World::load(&base)?;
    let proof = wire::verify(&old)?;
    let prepared = prepare::verify(&proof)?;
    let future = wire::World::load_transition(&c["prospective"])?;
    let futureproof = wire::verify_transition(&future, &old, &prepared.authored)?;
    for source in [false, true] {
        let mut next = admission::load_selected(&c, &base)?;
        if source {
            next.source.values_mut().next().unwrap()[0] ^= 1;
        } else {
            next.allocations.values_mut().next().unwrap().witness["inode"] = json!("9999");
        }
        assert!(matches!(
            admission::verify(&c, &old, &next, &future, &proof, &prepared, &futureproof),
            Err("admission no packed/source/journal effect")
        ));
    }
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, b| {
        write!(out, "{b:02x}").unwrap();
        out
    })
}
#[test]
fn coherent_unreachable_frame_cannot_replace_original_decoded_recipe() -> wire::Result<()> {
    use serde_json::json;
    let mut c = corpus();
    let id = "96000000-0000-4000-8000-000000001002";
    let mut raw = wire::unhex(c["prospective"]["allocations"][id]["hex"].as_str().unwrap())?;
    let mut p = 0usize;
    let mut last = 0;
    while p < raw.len() {
        last = p;
        p += 16
            + usize::try_from(u32::from_le_bytes(raw[p + 12..p + 16].try_into().unwrap())).unwrap();
    }
    assert_eq!(raw[last + 8], 0);
    let body = wire::encode(&prepare::corpus()["records"]["receipt-4"]["input"]);
    let mut suffix = b"PS2PKD01".to_vec();
    suffix.extend([1, 0, 0, 0]);
    suffix.extend(u32::try_from(body.len()).unwrap().to_le_bytes());
    suffix.extend(body);
    let gap = raw.len() - last - suffix.len() - 16;
    suffix.extend(b"PS2PKD01");
    suffix.extend([0, 0, 0, 0]);
    suffix.extend(u32::try_from(gap).unwrap().to_le_bytes());
    suffix.resize(raw.len() - last, 0);
    raw[last..].copy_from_slice(&suffix);
    c["prospective"]["allocations"][id]["hex"] = json!(hex(&raw));
    let descriptor = &mut c["dry_run"]["recipe"][0];
    let start = usize::try_from(wire::number(&descriptor["original_end"])?).unwrap();
    descriptor["framed_sha256"] = json!(wire::hash(&raw[start..]));
    descriptor["frame_count"] = json!((wire::number(&descriptor["frame_count"])? + 1).to_string());
    let changed = descriptor.clone();
    rebind(&mut c, |o, _| {
        o["payload_recipe"]["allocations"][0] = changed;
    });
    let base = original();
    let old = wire::World::load(&base)?;
    let oldproof = wire::verify(&old)?;
    let prepared = prepare::verify(&oldproof)?;
    let future = wire::World::load_transition(&c["prospective"])?;
    // Its selected reachable closure remains valid; original raw recipe still must win.
    wire::verify_transition(&future, &old, &prepared.authored)?;
    assert!(matches!(
        check(&c),
        Err("independent deterministic full append regeneration")
    ));
    Ok(())
}
