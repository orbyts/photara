//! Proposed accounting-scaling coordinates, independently generated; no production writes.
#[path = "accounting_scaling_candidate/accounting.rs"]
mod accounting;
#[path = "scalable_wire_candidate/wire.rs"]
#[allow(dead_code, reason = "Reuse exact candidate frame parser only")]
mod packed;
use accounting::{Kind, Result, Store, encode, ensure, reference};
use serde_json::{Value, json};
const BYTES: &str =
    include_str!("../../../docs/architecture/proposals/ps2/accounting/linked-accounting.json");
const PROFILE: &str = "example.ps2.synthetic-local-profile-v1";
const INC: &str = "71000000-0000-4000-8000-000000000001";
fn corpus() -> Value {
    serde_json::from_str(BYTES).unwrap()
}
#[test]
fn actual_proposed_bytes_select_multilevel_original_accounting() -> Result<()> {
    let c = corpus();
    let p = accounting::verify(&c, PROFILE, INC, true)?;
    ensure(
        p.total == 90112
            && p.observations == 21
            && p.retained == 18
            && p.depth >= 4
            && p.scope_matches,
        "actual original totals/depth",
    )?;
    let joined: Value = serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/joined/linked-joined.json"
    ))
    .unwrap();
    let store = Store::load(&c)?;
    let ledger = store.get(&c["ledger"])?;
    let conversion = store.get(&ledger["conversion_source"])?;
    ensure(
        joined["records"]
            .as_object()
            .unwrap()
            .values()
            .any(|r| r["canonical"].as_str().unwrap().as_bytes() == encode(&conversion)),
        "original conversion canonical bytes changed",
    )
}
#[test]
fn sparse_path_survives_source_and_all_unselected_node_removal() -> Result<()> {
    let mut c = corpus();
    let store = Store::load(&c)?;
    let ledger = store.get(&c["ledger"])?;
    let full = accounting::tree(&store, &ledger["sealed_charge_root"], Kind::Charge)?;
    ensure(
        c["sparse"]["nodes"].as_array().unwrap().len() < full.nodes.len(),
        "proof retained whole charge tree",
    )?;
    c["sealed_files"] = json!({});
    c["source_files"] = json!({});
    c["source_witnesses"] = json!({});
    c["records"] = json!({});
    ensure(
        accounting::sparse(&c["sparse"], &ledger["sealed_charge_root"], PROFILE, INC)? == 4096,
        "source-free rooted inclusion",
    )
}
#[test]
fn profile_or_incarnation_change_is_read_only_without_rebind() -> Result<()> {
    let c = corpus();
    for (profile, inc) in [
        ("example.other-profile", INC),
        (PROFILE, "71000000-0000-4000-8000-000000000099"),
    ] {
        ensure(
            !accounting::verify(&c, profile, inc, false)?.scope_matches,
            "mismatch silently writable",
        )?;
        ensure(
            accounting::verify(&c, profile, inc, true).is_err(),
            "mismatch mutation admitted",
        )?;
    }
    Ok(())
}

fn row(v: &Value) -> Value {
    use std::fmt::Write as _;
    let b = encode(v);
    let tag = if v["schema"]["id"] == "photara.package.conversion-source" {
        1
    } else {
        2
    };
    let mut frame = b"PS2PKD01".to_vec();
    frame.extend([tag, 0, 0, 0]);
    frame.extend(u32::try_from(b.len()).unwrap().to_le_bytes());
    frame.extend(&b);
    let frame_hex = frame.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").unwrap();
        out
    });
    json!({"input":v,"canonical":String::from_utf8(b.clone()).unwrap(),"sha256":packed::hash(&b),"byte_length":b.len().to_string(),"frame_hex":frame_hex,"frame_sha256":packed::hash(&frame)})
}
// Test-only coherent ancestor rehash: every replacement remains authenticated.
fn rewrite(c: &mut Value, target: &str, replacement: Value) {
    use std::collections::BTreeMap;
    fn node(
        hash: &str,
        objects: &BTreeMap<String, Value>,
        cache: &mut BTreeMap<String, Value>,
    ) -> Value {
        fn visit(
            v: &Value,
            objects: &BTreeMap<String, Value>,
            cache: &mut BTreeMap<String, Value>,
        ) -> Value {
            if v["kind"] == "json"
                && let Some(h) = v["sha256"].as_str()
                && objects.contains_key(h)
            {
                return reference(&node(h, objects, cache));
            }
            match v {
                Value::Object(m) => Value::Object(
                    m.iter()
                        .map(|(k, v)| (k.clone(), visit(v, objects, cache)))
                        .collect(),
                ),
                Value::Array(a) => {
                    Value::Array(a.iter().map(|v| visit(v, objects, cache)).collect())
                }
                _ => v.clone(),
            }
        }
        if let Some(v) = cache.get(hash) {
            return v.clone();
        }
        let v = visit(&objects[hash], objects, cache);
        cache.insert(hash.into(), v.clone());
        v
    }
    let mut objects: BTreeMap<_, _> = c["records"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v["input"].clone()))
        .collect();
    objects.insert(target.into(), replacement);
    let mut cache = BTreeMap::new();
    let root = node(
        c["ledger"]["sha256"].as_str().unwrap(),
        &objects,
        &mut cache,
    );
    c["ledger"] = reference(&root);
    c["records"] = Value::Object(
        cache
            .values()
            .map(|v| (reference(v)["sha256"].as_str().unwrap().to_owned(), row(v)))
            .collect(),
    );
}
fn find(c: &Value, schema: &str) -> (String, Value) {
    c["records"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, r)| r["input"]["schema"]["id"] == schema)
        .map(|(k, r)| (k.clone(), r["input"].clone()))
        .unwrap()
}
#[test]
fn coherent_order_range_count_overflow_and_schema_changes_refuse() -> Result<()> {
    for mode in ["order", "range", "count", "overflow", "version", "field"] {
        let mut c = corpus();
        let schema = if mode == "order" {
            "photara.package.retained-file-charge-leaf"
        } else {
            "photara.package.retained-file-charge-branch"
        };
        let (hash, mut v) = find(&c, schema);
        match mode {
            "order" => v["entries"].as_array_mut().unwrap().reverse(),
            "range" => v["children"][0]["last"] = v["children"][1]["last"].clone(),
            "count" => v["count"] = json!("999"),
            "overflow" => v["children"][0]["charged_high_water"] = json!(u64::MAX.to_string()),
            "version" => v["schema"]["version"] = json!(2),
            _ => v["unknown_required"] = json!(true),
        }
        rewrite(&mut c, &hash, v);
        ensure(
            accounting::verify(&c, PROFILE, INC, true).is_err(),
            "coherently authenticated invalid tree accepted",
        )?;
    }
    Ok(())
}
#[test]
fn coherent_original_charge_witness_or_logical_identity_swaps_refuse() -> Result<()> {
    for mode in ["undercharge", "witness", "logical", "observation"] {
        let mut c = corpus();
        let schema = if mode == "witness" {
            "photara.storage.local-observation"
        } else {
            "photara.package.retained-file-charge-leaf"
        };
        let (hash, mut v) = find(&c, schema);
        match mode {
            "undercharge" => v["entries"][0]["registered_charge"] = json!("0"),
            "witness" => v["physical"]["inode"] = json!("999999"),
            "logical" => v["entries"][0]["source_file"]["sha256"] = json!("0".repeat(64)),
            _ => v["entries"][0]["observation"] = v["entries"][1]["observation"].clone(),
        }
        rewrite(&mut c, &hash, v);
        ensure(
            accounting::verify(&c, PROFILE, INC, true).is_err(),
            "coherent accounting swap accepted",
        )?;
    }
    Ok(())
}
#[test]
fn independent_tree_closures_do_not_borrow_nodes() -> Result<()> {
    let c = corpus();
    let mut store = Store::load(&c)?;
    let ledger = store.get(&c["ledger"])?;
    let missing = store
        .objects
        .iter()
        .find(|(_, v)| v["schema"]["id"] == "photara.package.retained-file-charge-leaf")
        .unwrap()
        .0
        .clone();
    store.objects.remove(&missing);
    accounting::tree(&store, &ledger["observation_root"], Kind::Observation)?;
    accounting::tree(&store, &ledger["sealed_charge_root"], Kind::Charge)?;
    ensure(
        accounting::tree(&store, &ledger["retained_file_charge_root"], Kind::Retained).is_err(),
        "missing typed closure borrowed",
    )
}
#[test]
fn sparse_root_sibling_charge_and_witness_swaps_refuse() -> Result<()> {
    let c = corpus();
    let root = c["sparse"]["root"].clone();
    for mode in ["root", "sibling", "charge", "witness", "extra"] {
        let mut p = c["sparse"].clone();
        match mode {
            "root" => p["root"]["sha256"] = json!("0".repeat(64)),
            "sibling" => {
                let mut v = p["nodes"][0]["input"].clone();
                v["children"][1]["child"]["sha256"] = json!("0".repeat(64));
                p["nodes"][0] = row(&v);
            }
            "charge" => {
                let mut v = p["charge"]["input"].clone();
                v["charged_high_water"] = json!("8192");
                p["charge"] = row(&v);
            }
            "witness" => {
                let mut v = p["observation"]["input"].clone();
                v["physical"]["inode"] = json!("999");
                p["observation"] = row(&v);
            }
            _ => {
                let duplicate = p["nodes"][0].clone();
                p["nodes"].as_array_mut().unwrap().push(duplicate);
            }
        }
        ensure(
            accounting::sparse(&p, &root, PROFILE, INC).is_err(),
            "sparse original commitment substitution accepted",
        )?;
    }
    Ok(())
}

#[test]
fn actual_new_schema_dispatch_and_original_conversion_identity_refuse_changes() -> Result<()> {
    let original = corpus();
    for schema in [
        "photara.storage.local-observation",
        "photara.storage.local-observation-leaf",
        "photara.storage.local-observation-branch",
        "photara.storage.sealed-charge",
        "photara.storage.charge-leaf",
        "photara.storage.charge-branch",
        "photara.package.retained-file-charge-leaf",
        "photara.package.retained-file-charge-branch",
    ] {
        for version in [false, true] {
            let mut c = original.clone();
            let (hash, mut v) = find(&c, schema);
            if version {
                v["schema"]["version"] = json!(2);
            } else {
                v["unknown_required"] = json!(true);
            }
            rewrite(&mut c, &hash, v);
            ensure(
                accounting::verify(&c, PROFILE, INC, true).is_err(),
                "actual new codec accepted unknown layout",
            )?;
        }
    }
    let mut c = original.clone();
    let (hash, mut v) = find(&c, "photara.package.conversion-source");
    v["files"].as_array_mut().unwrap().reverse();
    rewrite(&mut c, &hash, v);
    ensure(
        accounting::verify(&c, PROFILE, INC, true).is_err(),
        "original ConversionSource reserialized/replaced",
    )?;
    let mut c = original;
    let (hash, mut v) = find(&c, "photara.storage.local-observation");
    v["subject"]["kind"] = json!("whole-blob");
    rewrite(&mut c, &hash, v);
    ensure(
        accounting::verify(&c, PROFILE, INC, true).is_err(),
        "unimplemented whole-blob variant silently accepted",
    )
}
#[test]
fn equal_content_at_distinct_component_paths_keeps_two_charges() -> Result<()> {
    let c = corpus();
    let (_, mut leaf) = find(&c, "photara.package.retained-file-charge-leaf");
    leaf["entries"][1]["source_file"] = leaf["entries"][0]["source_file"].clone();
    let r = reference(&leaf);
    let store = Store {
        objects: std::collections::BTreeMap::from([(
            r["sha256"].as_str().unwrap().to_owned(),
            leaf,
        )]),
    };
    let p = accounting::tree(&store, &r, Kind::Retained)?;
    ensure(
        p.entries.len() == 2
            && p.charge == 8192
            && p.entries[0]["conversion_source"] == p.entries[1]["conversion_source"],
        "same content collapsed distinct original file identities",
    )
}
#[test]
fn sparse_structural_sibling_summaries_checked_even_under_rehashed_root() -> Result<()> {
    let c = corpus();
    for mode in ["range", "overflow", "count"] {
        let mut proof = c["sparse"].clone();
        let mut root = proof["nodes"][0]["input"].clone();
        match mode {
            "range" => root["children"][1]["first"] = root["children"][0]["last"].clone(),
            "overflow" => root["children"][0]["charged_high_water"] = json!(u64::MAX.to_string()),
            _ => root["count"] = json!("999"),
        }
        proof["nodes"][0] = row(&root);
        proof["root"] = reference(&root);
        ensure(
            accounting::sparse(&proof, &proof["root"], PROFILE, INC).is_err(),
            "sparse structural summary accepted",
        )?;
    }
    Ok(())
}

#[test]
fn original_path_grammar_and_actual_sealed_bytes_are_preserved() -> Result<()> {
    for component in ["CON", "trailing.", "trailing ", "control\u{0001}"] {
        let mut c = corpus();
        let (hash, mut leaf) = find(&c, "photara.package.retained-file-charge-leaf");
        leaf["entries"][0]["key"]["path"] = json!([component]);
        rewrite(&mut c, &hash, leaf);
        ensure(
            accounting::verify(&c, PROFILE, INC, true).is_err(),
            "weakened original path grammar",
        )?;
    }
    let mut c = corpus();
    let id = c["sealed_files"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let mut raw = packed::unhex(c["sealed_files"][&id]["hex"].as_str().unwrap())?;
    raw[20] ^= 1;
    let hex = raw.iter().fold(String::new(), |mut out, b| {
        use std::fmt::Write as _;
        write!(out, "{b:02x}").unwrap();
        out
    });
    c["sealed_files"][&id]["hex"] = json!(hex);
    c["sealed_files"][&id]["sha256"] = json!(packed::hash(&raw));
    ensure(
        accounting::verify(&c, PROFILE, INC, true).is_err(),
        "same-size rehashed original allocation replacement",
    )
}

#[test]
fn optional_extension_reference_shape_is_not_an_accounting_edge() -> Result<()> {
    let mut c = corpus();
    let (hash, mut observation) = find(&c, "photara.storage.local-observation");
    observation["extensions"] =
        json!({"example.opaque":{"kind":"json","sha256":"0".repeat(64),"byte_length":"99"}});
    rewrite(&mut c, &hash, observation);
    ensure(
        accounting::verify(&c, PROFILE, INC, true)?.total == 90112,
        "opaque reference added to selected closure",
    )
}
