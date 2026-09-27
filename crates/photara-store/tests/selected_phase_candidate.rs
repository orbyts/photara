//! Same-original selected byte phases. Pure process-cut model, never native durability.
#![allow(dead_code, reason = "Shared candidate verification entry points")]
#[path = "integrated_wire_candidate/accounting.rs"]
mod accounting;
#[path = "selected_admission_candidate/admission.rs"]
mod admission;
#[path = "selected_admission_candidate/decoder.rs"]
mod decoder;
#[path = "selected_phase_candidate/phases.rs"]
mod phases;
#[path = "selected_admission_candidate/prepare.rs"]
mod prepare;
#[path = "selected_phase_candidate/replay_prepare.rs"]
mod replay_prepare;
#[path = "integrated_wire_candidate/resources.rs"]
mod resources;
#[path = "integrated_wire_candidate/wire.rs"]
mod wire;
fn corpus() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/selected-phase/linked.json"
    ))
    .unwrap()
}
fn world(stage: &str) -> wire::World {
    let c = corpus();
    let mut v = c["snapshots"][stage].clone();
    v["allocations"] = c["allocations"].clone();
    v["source_files"] = c["source_files"].clone();
    v["source_witnesses"] = c["source_witnesses"].clone();
    for (id, a) in v["allocations"].as_object_mut().unwrap() {
        let n = c["snapshots"][stage]["ends"][id]
            .as_str()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        a["hex"] = serde_json::json!(&a["hex"].as_str().unwrap()[..n * 2]);
    }
    wire::World::load_transition(&v).unwrap()
}
#[test]
fn actual_selected_same_original_phase_route() {
    for (i, stage) in [
        "admitted",
        "journal-durable",
        "payload-durable",
        "finalizer-selected",
        "finalizer-durable",
        "published",
        "clean",
    ]
    .iter()
    .enumerate()
    {
        let w = world(stage);
        let p = phases::verify(&w).unwrap_or_else(|e| panic!("{stage}: {e}"));
        assert_eq!(p.stage, i);
        assert_eq!(p.receipts.len(), if i >= 5 { 4 } else { 3 });
        assert!(p.control_peak <= 131_072);
        assert_eq!(p.consumed, if i >= 5 { 1_835_008 } else { 0 });
    }
}
#[test]
fn fresh_handles_replay_matching_partial_payload_and_finalizer_only() {
    for (stage, target) in [
        ("journal-durable", "payload-durable"),
        ("finalizer-selected", "finalizer-durable"),
    ] {
        let start = world(stage);
        let complete = world(target);
        for id in [
            "96000000-0000-4000-8000-000000001002",
            "96000000-0000-4000-8000-000000001008",
        ] {
            let before = start.allocations[id].bytes.len();
            let after = complete.allocations[id].bytes.len();
            if before == after {
                continue;
            }
            for n in [before + 1, before + (after - before) / 2, after] {
                let mut partial = start.clone();
                partial.allocations.get_mut(id).unwrap().bytes =
                    complete.allocations[id].bytes[..n].to_vec();
                let resumed = phases::advance(&partial).unwrap();
                assert_eq!(resumed.head, complete.head);
                assert_eq!(
                    resumed.allocations[id].bytes,
                    complete.allocations[id].bytes
                );
            }
        }
    }
    let mut early = world("payload-durable");
    let id = "96000000-0000-4000-8000-000000001002";
    let complete = world("finalizer-durable");
    let n = early.allocations[id].bytes.len();
    early
        .allocations
        .get_mut(id)
        .unwrap()
        .bytes
        .push(complete.allocations[id].bytes[n]);
    assert_eq!(
        phases::advance(&early).err(),
        Some("original-authorized exact live suffix")
    );
}
fn edit_control(w: &mut wire::World, name: &str, change: impl FnOnce(&mut serde_json::Value)) {
    fn replace(v: &mut serde_json::Value, old: &serde_json::Value, new: &serde_json::Value) {
        if *v == *old {
            *v = new.clone();
            return;
        }
        match v {
            serde_json::Value::Object(m) => {
                for v in m.values_mut() {
                    replace(v, old, new);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    replace(v, old, new);
                }
            }
            _ => {}
        }
    }
    let mut values = w
        .loose
        .values()
        .map(|b| wire::parse(b).unwrap())
        .collect::<Vec<_>>();
    let i = values
        .iter()
        .position(|v| v["schema"]["id"] == name)
        .unwrap();
    let old = wire::reference(&values[i]);
    change(&mut values[i]);
    let mut queue = vec![(old, wire::reference(&values[i]))];
    let mut n = 0;
    while n < queue.len() {
        assert!(n < 64);
        let (old, new) = queue[n].clone();
        for v in &mut values {
            let r = wire::reference(v);
            replace(v, &old, &new);
            let changed = wire::reference(v);
            if r != changed {
                queue.push((r, changed));
            }
        }
        replace(&mut w.commit, &old, &new);
        n += 1;
    }
    w.loose = values
        .iter()
        .map(|v| (wire::hash(&wire::encode(v)), wire::encode(v)))
        .collect();
    w.rehash_head();
}
#[test]
fn changed_original_phase_controls_unknown_suffix_and_no_effects_refuse() {
    for (stage, field, value) in [
        ("published", "consumed", "1835009"),
        ("published", "remaining", "65535"),
        ("clean", "remaining", "65536"),
    ] {
        let mut w = world(stage);
        edit_control(&mut w, "photara.storage.operation-phase", |p| {
            p[field] = serde_json::json!(value);
        });
        assert_eq!(
            phases::advance(&w).err(),
            Some("exact original-derived selected phase controls")
        );
    }
    let mut clean = world("clean");
    edit_control(&mut clean, "photara.storage.operation-phase", |p| {
        p["cleanup"]["released"] = serde_json::json!("65537");
    });
    assert_eq!(
        phases::advance(&clean).err(),
        Some("exact original-derived selected phase controls")
    );
    let mut codec = world("admitted");
    edit_control(&mut codec, "photara.storage.original-admission", |o| {
        o["original_codec"] = serde_json::json!("future-unknown");
    });
    assert_eq!(
        phases::advance(&codec).err(),
        Some("original replay codec supported")
    );
    let mut unknown = world("finalizer-selected");
    unknown
        .allocations
        .get_mut("96000000-0000-4000-8000-000000001002")
        .unwrap()
        .bytes
        .push(255);
    assert_eq!(
        phases::advance(&unknown).err(),
        Some("original-authorized exact live suffix")
    );
    let mut extra = world("published");
    extra.loose.insert("f".repeat(64), b"foreign".to_vec());
    assert_eq!(
        phases::advance(&extra).err(),
        Some("exact original-derived selected phase controls")
    );
}
#[test]
fn clean_retry_returns_original_receipt_without_second_charge_or_release() {
    let w = world("clean");
    let (o, _) = phases::selected_original(&w).unwrap();
    let r = &o["request"];
    for _ in 0..2 {
        let (receipt, charge, release) = phases::retry(
            &w,
            &wire::reference(&o),
            r["request_sha256"].as_str().unwrap(),
        )
        .unwrap();
        assert_eq!(wire::reference(&receipt), r["receipt"]);
        assert_eq!((charge, release), (0, 0));
    }
    let mut other = wire::reference(&o);
    other["sha256"] = serde_json::json!("f".repeat(64));
    assert_eq!(
        phases::retry(&w, &other, r["request_sha256"].as_str().unwrap()).err(),
        Some("exact original terminal retry")
    );
}
#[test]
fn recovery_reads_current_original_root_without_active_or_retained_payload() {
    for stage in ["published", "clean"] {
        let mut w = world(stage);
        for n in [1002, 1003, 1006, 1007] {
            w.allocations
                .remove(&format!("96000000-0000-4000-8000-{n:012}"));
        }
        assert_eq!(phases::recovery(&w).unwrap().receipts.len(), 3);
        assert_eq!(
            phases::advance(&w).err(),
            Some("original allocation absent")
        );
        let mut wrong = w.clone();
        wrong
            .allocations
            .get_mut("96000000-0000-4000-8000-000000001004")
            .unwrap()
            .witness["inode"] = serde_json::json!("999");
        assert_eq!(
            phases::recovery(&wrong).err(),
            Some("readonly supplied original suffix")
        );
    }
}
#[test]
fn each_selector_cut_reconciles_exact_original_and_preserves_unknown_controls() {
    let c = corpus();
    let stages = [
        "admitted",
        "journal-durable",
        "payload-durable",
        "finalizer-selected",
        "finalizer-durable",
        "published",
        "clean",
    ];
    for pair in stages.windows(2) {
        let mut before = world(pair[0]);
        let after = world(pair[1]);
        before.allocations = after.allocations.clone();
        before.journal.clone_from(&after.journal);
        let sidecars = c["selector_cuts"][pair[0]]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap().as_bytes().to_vec()))
            .collect::<std::collections::BTreeMap<_, _>>();
        for (selected, staged) in [(&before, false), (&before, true), (&after, true)] {
            let mut artifacts = sidecars.clone();
            if !staged {
                artifacts.remove("HEAD.next");
            }
            let result = phases::reconcile(selected, &artifacts).unwrap();
            assert_eq!(result.head, after.head);
            assert_eq!(result.loose, after.loose);
        }
    }
    let before = world("published");
    let mut sidecars = c["selector_cuts"]["published"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().as_bytes().to_vec()))
        .collect::<std::collections::BTreeMap<_, _>>();
    sidecars.insert("unknown.control".into(), b"retain me".to_vec());
    assert_eq!(
        phases::reconcile(&before, &sidecars).err(),
        Some("selector exact known sidecar set")
    );
    assert_eq!(sidecars["unknown.control"], b"retain me");
}
#[test]
fn coherent_recovery_original_metadata_refuses_without_foreign_payload_authority() {
    for (field, value, error) in [
        ("token", "0", "original metadata graph token"),
        ("kind", "retire", "original metadata graph token"),
        ("nonce", "not-hex", "original metadata nonce"),
        ("project_limit", "1", "original metadata reserve capacity"),
    ] {
        let bad = phases::rebind_original_negative(&world("clean"), |o| {
            o[field] = serde_json::json!(value);
        })
        .unwrap();
        assert_eq!(phases::recovery(&bad).err(), Some(error), "{field}");
    }
    for (which, error) in [
        (0, "original metadata local scope"),
        (1, "original metadata retained state identity"),
        (2, "original metadata exact target generation"),
    ] {
        let bad = phases::rebind_original_negative(&world("published"), |o| match which {
            0 => o["scope"]["directory"]["inode"] = serde_json::json!("100"),
            1 => o["old"]["states"][0]["value"]["authored_revision"] = serde_json::json!("999"),
            _ => o["payload_recipe"]["target_placement"]["generation"] = serde_json::json!("3"),
        })
        .unwrap();
        assert_eq!(phases::recovery(&bad).err(), Some(error));
    }
}
#[test]
fn selector_cleanup_cannot_invent_missing_journal_or_candidate_control_bytes() {
    let c = corpus();
    let w = world("admitted");
    let sidecars = c["selector_cuts"]["admitted"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().as_bytes().to_vec()))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        phases::reconcile(&w, &sidecars).err(),
        Some("selector observed journal prerequisites")
    );
    let mut ready = w.clone();
    ready.journal = world("journal-durable").journal;
    let mut missing = sidecars.clone();
    let key = missing
        .keys()
        .find(|k| k.starts_with("control/"))
        .unwrap()
        .clone();
    missing.remove(&key);
    assert_eq!(
        phases::reconcile(&ready, &missing).err(),
        Some("selector exact known sidecar set")
    );
    ready.journal.last_mut().unwrap()["receipt_sha256"] = serde_json::json!("f".repeat(64));
    assert_eq!(
        phases::reconcile(&ready, &sidecars).err(),
        Some("phase exact source/journal preservation")
    );
}
#[test]
fn recovery_checks_embedded_receipt_manifest_set_pin_set_and_individual_cap() {
    for (mode, error) in [
        (0, "exact fields"),
        (1, "original finalizer byte bound"),
        (2, "readonly sorted unique original manifests"),
        (3, "readonly exact original pinned state set"),
    ] {
        let bad = phases::rebind_evidence_negative(&world("clean"), |o, f| match mode {
            0 => {
                f["prepared_receipt"]["unknown_required"] = serde_json::json!(true);
                o["request"]["receipt"] = wire::reference(&f["prepared_receipt"]);
            }
            1 => {
                f["extensions"]["example.oversized"] = serde_json::json!("x".repeat(16_384));
            }
            2 => {
                o["payload_recipe"]["allocations"]
                    .as_array_mut()
                    .unwrap()
                    .swap(0, 1);
                f["payload_completion"].as_array_mut().unwrap().swap(0, 1);
                f["writes"].as_array_mut().unwrap().swap(0, 1);
            }
            _ => {
                let active = o["old"]["commit"]["root_set"]["active"].clone();
                let recovery = o["old"]["commit"]["root_set"]["recovery"].clone();
                let row = o["old"]["states"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["root"] != active && r["root"] != recovery)
                    .unwrap();
                row["value"]["root_id"] = serde_json::json!("96000000-0000-4000-8000-000000008888");
                row["root"] = wire::reference(&row["value"]);
            }
        })
        .unwrap();
        assert_eq!(phases::recovery(&bad).err(), Some(error), "mode {mode}");
    }
}
