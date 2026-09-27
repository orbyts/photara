//! New-only, pure-memory candidate branch schemas and independently fixed bytes.
#[path = "scalable_operation_candidate/operations.rs"]
#[allow(dead_code, reason = "Use independent real-operation closure oracle")]
mod operations;
#[path = "scalable_wire_candidate/wire.rs"]
#[allow(dead_code, reason = "Reuse established exact packed-frame parser")]
mod packed;
#[path = "scalable_branch_candidate/trees.rs"]
mod trees;
use packed::{Result, ensure, hash};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use trees::{Key, Kind, Store, encode, key, number, tree};
const SOURCE: &str =
    include_str!("../../../docs/architecture/proposals/ps2/operations/linked-operations.json");
const CORPUS: &str =
    include_str!("../../../docs/architecture/proposals/ps2/branches/linked-branches.json");
fn corpus() -> Value {
    serde_json::from_str(CORPUS).unwrap()
}
fn source() -> Value {
    serde_json::from_str(SOURCE).unwrap()
}
fn setup() -> Store {
    Store::load(&corpus()).unwrap()
}

/// Derive expected semantic membership from the already verified actual operation
/// specimen; replace only the old leaf indexes with the new traversed structures.
fn role(store: &Store, role: &str) -> Result<BTreeSet<Key>> {
    let old = operations::Store::load(&source())?;
    let proof = operations::verify_role(&old, role)?;
    let selected = &store.selector["roles"][role];
    let mut expected = proof.closure;
    let old_index = old.get(&old.roots[role]["operation_index"])?;
    for r in [
        &old.roots[role]["operation_index"],
        &old_index["by_id"],
        &old_index["by_ordinal"],
    ] {
        expected.remove(&key(r)?);
    }
    let new_index = store.get(&selected["operation_index"], 1)?;
    let mut normalized = new_index.clone();
    for (field, kind) in [("by_id", Kind::Id), ("by_ordinal", Kind::Ordinal)] {
        let p = tree(store, &new_index[field], kind)?;
        ensure(
            Some(&p.entries) == old.get(&old_index[field])?["entries"].as_array(),
            "reciprocal original operation entries",
        )?;
        expected.extend(p.nodes);
        normalized[field] = old_index[field].clone();
    }
    ensure(
        normalized == old_index,
        "operation root identity and accepted prefix",
    )?;
    expected.insert(key(&selected["operation_index"])?);
    let mut original_selected = selected.clone();
    original_selected
        .as_object_mut()
        .ok_or("selected role")?
        .remove("inventory");
    original_selected["operation_index"] = old.roots[role]["operation_index"].clone();
    ensure(
        original_selected == old.roots[role],
        "original role coordinates",
    )?;
    // Original semantic objects must be present and byte-identical in this pack;
    // no receipt digest becomes a storage edge and no external fallback is used.
    for k in &operations::verify_role(&old, role)?.closure {
        if expected.contains(k) {
            let r = json!({"kind":"json","sha256":k.0,"byte_length":k.1.to_string()});
            ensure(
                store.get(&r, 1)? == old.get(&r)?,
                "original packed semantic bytes",
            )?;
        }
    }
    let inventory = tree(store, &selected["inventory"], Kind::Inventory)?;
    let inventoried = inventory
        .entries
        .iter()
        .map(key)
        .collect::<Result<BTreeSet<_>>>()?;
    ensure(inventoried == expected, "exact semantic inventory coverage")?;
    expected.extend(inventory.nodes);
    Ok(expected)
}
fn charges(store: &Store) -> Result<BTreeSet<Key>> {
    let proof = tree(store, &store.selector["owned_charges"], Kind::Charge)?;
    let mut nodes = proof.nodes;
    let observations = store.selector["sealed_observations"]
        .as_array()
        .ok_or("observations")?;
    ensure(
        observations.len() == proof.entries.len(),
        "exact charge observation coverage",
    )?;
    let mut previous = None;
    for (entry, observed) in proof.entries.iter().zip(observations) {
        ensure(
            previous
                .as_ref()
                .is_none_or(|p| p < &entry["allocation_id"].as_str().unwrap().to_string()),
            "unique charge IDs",
        )?;
        previous = Some(entry["allocation_id"].as_str().unwrap().to_string());
        let value = store.get(&entry["charge"], 2)?;
        let expected = json!({"schema":{"id":"example.ps2.sealed-charge","version":1},"project_id":"10000000-0000-4000-8000-000000000001","extensions":{},"allocation_id":observed["allocation_id"],"arena":observed["arena"],"domain_incarnation":observed["domain_incarnation"],"measured_extent":observed["measured_extent"],"charged_high_water":observed["charged_high_water"],"observation":value["observation"]});
        ensure(
            value == expected
                && value["allocation_id"] == entry["allocation_id"]
                && value["domain_incarnation"] == store.selector["domain_incarnation"]
                && value["charged_high_water"] == entry["charged_high_water"]
                && number(&value["charged_high_water"])? >= number(&value["measured_extent"])?
                && value["observation"]["kind"] == "synthetic-vector",
            "exact observed sealed charge",
        )?;
        nodes.insert(key(&entry["charge"])?);
    }
    let mut total = proof.charge;
    let mut tips = BTreeSet::new();
    for tip in store.selector["tips"].as_array().ok_or("tips")? {
        let id = tip["allocation_id"].as_str().ok_or("tip identity")?;
        ensure(
            tips.insert(id) && !observations.iter().any(|v| v["allocation_id"] == id),
            "disjoint unique canonical allocations",
        )?;
        total = total
            .checked_add(number(&tip["charged_high_water"])?)
            .ok_or("accounting overflow")?;
    }
    ensure(
        total == number(&store.selector["total_charge"])?,
        "once per allocation charge",
    )?;
    Ok(nodes)
}
fn verify(store: &Store) -> Result<BTreeSet<Key>> {
    trees::dispatch(&store.selector)?;
    let mut all = role(store, "active")?;
    all.extend(role(store, "recovery")?);
    all.extend(charges(store)?);
    ensure(
        all == store.objects.keys().cloned().collect(),
        "exact semantic and owner locator membership",
    )?;
    Ok(all)
}
#[test]
fn fixed_canonical_tree_bytes_and_physical_frames_match_independent_generator() {
    let c = corpus();
    assert_eq!(c["operation_corpus_sha256"], hash(SOURCE.as_bytes()));
    for row in c["records"]
        .as_object()
        .unwrap()
        .values()
        .chain([&c["locator"]])
    {
        let bytes = encode(&row["input"]);
        assert_eq!(bytes, row["canonical_utf8"].as_str().unwrap().as_bytes());
        assert_eq!(hash(&bytes), row["sha256"]);
    }
    let store = Store::load(&c).unwrap();
    assert_eq!(verify(&store).unwrap().len(), 75);
    assert_eq!(number(&store.selector["total_charge"]).unwrap(), 155_648);
}
#[test]
fn genuine_multi_level_trees_preserve_real_operations_and_independent_recovery() {
    let store = setup();
    verify(&store).unwrap();
    for role_name in ["active", "recovery"] {
        let inventory = tree(
            &store,
            &store.selector["roles"][role_name]["inventory"],
            Kind::Inventory,
        )
        .unwrap();
        assert!(inventory.depth >= 4);
        let root = store
            .get(&store.selector["roles"][role_name]["operation_index"], 1)
            .unwrap();
        for (field, kind) in [("by_id", Kind::Id), ("by_ordinal", Kind::Ordinal)] {
            let p = tree(&store, &root[field], kind).unwrap();
            assert!(p.depth >= 2);
            if role_name == "active" {
                assert_eq!(p.depth, 3);
            }
        }
    }
    assert_eq!(
        tree(&store, &store.selector["owned_charges"], Kind::Charge)
            .unwrap()
            .depth,
        4
    );
    let recovery = role(&store, "recovery").unwrap();
    let mut isolated = store.clone();
    isolated.objects.retain(|k, _| recovery.contains(k));
    role(&isolated, "recovery").unwrap();
    assert!(role(&isolated, "active").is_err());
}
fn original_entry() -> Value {
    let old = operations::Store::load(&source()).unwrap();
    let index = old.get(&old.roots["active"]["operation_index"]).unwrap();
    old.get(&index["by_ordinal"]).unwrap()["entries"][0].clone()
}
fn modified_root(
    store: &mut Store,
    root: &Value,
    kind: Kind,
    edit: impl FnOnce(&mut Value),
) -> Value {
    let tag = if kind == Kind::Charge { 2 } else { 1 };
    let mut v = store.get(root, tag).unwrap();
    edit(&mut v);
    store.append(&v, tag)
}
#[test]
fn exact_child_ranges_counts_and_coverage_reject_corruption() {
    for kind in [Kind::Inventory, Kind::Id, Kind::Ordinal, Kind::Charge] {
        let base = setup();
        let root = match kind {
            Kind::Inventory => base.selector["roles"]["active"]["inventory"].clone(),
            Kind::Charge => base.selector["owned_charges"].clone(),
            _ => base
                .get(&base.selector["roles"]["active"]["operation_index"], 1)
                .unwrap()[if kind == Kind::Id {
                "by_id"
            } else {
                "by_ordinal"
            }]
            .clone(),
        };
        for mutation in 0..4 {
            let mut store = base.clone();
            let r = modified_root(&mut store, &root, kind, |v| match mutation {
                0 => v["children"][0]["last"] = v["children"][1]["last"].clone(),
                1 => v["count"] = json!("99"),
                2 => v["children"].as_array_mut().unwrap().reverse(),
                _ => {
                    v["children"][0]["count"] = json!(u64::MAX.to_string());
                    v["children"][1]["count"] = json!("1");
                }
            });
            if mutation == 3 {
                assert_eq!(
                    tree(&store, &r, kind).err(),
                    Some("tree aggregate overflow")
                );
            } else {
                assert!(tree(&store, &r, kind).is_err(), "{kind:?}/{mutation}");
            }
        }
    }
    let mut store = setup();
    let root = store.selector["roles"]["active"]["inventory"].clone();
    let r = modified_root(&mut store, &root, Kind::Inventory, |v| {
        v["children"].as_array_mut().unwrap().pop();
        v["count"] = v["children"][0]["count"].clone();
    });
    store.selector["roles"]["active"]["inventory"] = r;
    assert!(role(&store, "active").is_err());
}
#[test]
fn aliases_duplicate_entries_and_impossible_content_addressed_cycles_refuse() {
    let mut store = setup();
    let root = store.selector["owned_charges"].clone();
    let duplicate = modified_root(&mut store, &root, Kind::Charge, |v| {
        v["children"][1] = v["children"][0].clone();
        v["count"] = json!((number(&v["children"][0]["count"]).unwrap() * 2).to_string());
        v["charged_high_water"] =
            json!((number(&v["children"][0]["charged_high_water"]).unwrap() * 2).to_string());
    });
    assert_eq!(
        tree(&store, &duplicate, Kind::Charge).err(),
        Some("tree alias/cycle")
    );
    let k = key(&root).unwrap();
    store.objects.get_mut(&k).unwrap().0["children"][0]["child"] = root.clone();
    assert_eq!(
        tree(&store, &root, Kind::Charge).err(),
        Some("typed bytes/identity")
    );
    let mut store = setup();
    let leaf = json!({"schema":{"id":"example.ps2.operation-ordinal-leaf","version":1},"project_id":"10000000-0000-4000-8000-000000000001","extensions":{},"count":"2","entries":[original_entry(),original_entry()]});
    let r = store.append(&leaf, 1);
    assert!(tree(&store, &r, Kind::Ordinal).is_err());
}
#[test]
fn charge_summaries_identity_and_once_only_accounting_are_exact() {
    for mutation in 0..4 {
        let mut store = setup();
        let root = store.selector["owned_charges"].clone();
        match mutation {
            0 => {
                let r = modified_root(&mut store, &root, Kind::Charge, |v| {
                    v["charged_high_water"] = json!("1");
                });
                store.selector["owned_charges"] = r;
            }
            1 => store.selector["sealed_observations"][0]["measured_extent"] = json!("8192"),
            2 => {
                store.selector["tips"][0]["allocation_id"] =
                    store.selector["sealed_observations"][0]["allocation_id"].clone();
            }
            _ => store.selector["total_charge"] = json!("155649"),
        }
        assert!(charges(&store).is_err());
    }
    let mut store = setup();
    store.selector["tips"][0]["charged_high_water"] = json!(u64::MAX.to_string());
    assert!(charges(&store).is_err());
}
#[test]
fn compatibility_dispatch_is_explicit_and_extensions_are_nonedges() {
    for mutation in 0..5 {
        let mut store = setup();
        match mutation {
            0 => {
                store.selector["required_features"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            1 => store.selector["required_features"][1] = json!("unknown.required.v1"),
            2 => store.selector["minimum_reader"]["minor"] = json!(2),
            3 => store.selector["kind"] = json!("flat"),
            _ => store.selector["required_features"]
                .as_array_mut()
                .unwrap()
                .push(json!(trees::FEATURE)),
        }
        assert!(verify(&store).is_err());
    }
    let mut store = setup();
    let root = store.selector["roles"]["active"]["inventory"].clone();
    for mutation in 0..3 {
        let r = modified_root(&mut store, &root, Kind::Inventory, |v| match mutation {
            0 => v["schema"]["version"] = json!(2),
            1 => v["schema"]["id"] = json!("photara.package.inventory"),
            _ => v["required_unknown"] = json!(true),
        });
        assert!(tree(&store, &r, Kind::Inventory).is_err());
    }
    let r = modified_root(&mut store, &root, Kind::Inventory, |v| {
        v["extensions"] =
            json!({"optional.example":{"kind":"json","sha256":"e".repeat(64),"byte_length":"999"}});
    });
    assert_eq!(
        tree(&store, &r, Kind::Inventory).unwrap().entries,
        tree(&store, &root, Kind::Inventory).unwrap().entries
    );
}
#[test]
fn exact_locator_membership_and_owned_tree_storage_are_required() {
    let mut store = setup();
    store.append(&json!({"unreachable":true}), 2);
    assert!(verify(&store).is_err());
    let mut c = corpus();
    c["selector"]["tips"].as_array_mut().unwrap().pop();
    assert!(Store::load(&c).is_err());
    let mut store = setup();
    let root = store.selector["owned_charges"].clone();
    store.objects.get_mut(&key(&root).unwrap()).unwrap().1 = 1;
    assert!(verify(&store).is_err());
    let mut store = setup();
    store.objects.remove(&key(&root).unwrap());
    assert!(verify(&store).is_err());
}

fn rewrite_first(store: &mut Store, root: &Value, kind: Kind, edit: &dyn Fn(&mut Value)) -> Value {
    let tag = if kind == Kind::Charge { 2 } else { 1 };
    let mut node = store.get(root, tag).unwrap();
    if node["entries"].is_array() {
        edit(&mut node["entries"][0]);
    } else {
        node["children"][0]["child"] =
            rewrite_first(store, &node["children"][0]["child"], kind, edit);
    }
    store.append(&node, tag)
}
#[test]
fn structurally_valid_wrong_membership_and_reciprocal_receipt_refuse() {
    let mut store = setup();
    let inventory = store
        .get(&store.selector["roles"]["active"]["inventory"], 1)
        .unwrap();
    let subset = inventory["children"][0]["child"].clone();
    tree(&store, &subset, Kind::Inventory).unwrap();
    store.selector["roles"]["active"]["inventory"] = subset;
    assert_eq!(
        role(&store, "active").err(),
        Some("exact semantic inventory coverage")
    );
    let mut store = setup();
    let mut index = store
        .get(&store.selector["roles"]["active"]["operation_index"], 1)
        .unwrap();
    index["by_id"] = rewrite_first(&mut store, &index["by_id"], Kind::Id, &|entry| {
        entry["request_sha256"] = json!("e".repeat(64));
    });
    tree(&store, &index["by_id"], Kind::Id).unwrap();
    let r = store.append(&index, 1);
    store.selector["roles"]["active"]["operation_index"] = r;
    assert_eq!(
        role(&store, "active").err(),
        Some("reciprocal original operation entries")
    );
}

#[test]
fn distinct_charge_leaf_values_refuse_exact_addition_overflow() {
    let mut store = setup();
    let proof = tree(&store, &store.selector["owned_charges"], Kind::Charge).unwrap();
    let mut entries = proof.entries[..2].to_vec();
    for (entry, value) in entries.iter_mut().zip([u64::MAX, 1]) {
        let mut charge = store.get(&entry["charge"], 2).unwrap();
        charge["measured_extent"] = json!("1");
        charge["charged_high_water"] = json!(value.to_string());
        entry["charge"] = store.append(&charge, 2);
        entry["charged_high_water"] = json!(value.to_string());
    }
    let leaf = json!({"schema":{"id":"example.ps2.owned-charge-leaf","version":1},"project_id":"10000000-0000-4000-8000-000000000001","extensions":{},"count":"2","entries":entries,"charged_high_water":"0"});
    let r = store.append(&leaf, 2);
    assert_eq!(
        tree(&store, &r, Kind::Charge).err(),
        Some("tree aggregate overflow")
    );
}
