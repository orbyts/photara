//! Actual one-HEAD overlay reader. All historical controls in O.old are nonedge
//! reconstruction evidence; no historical allocation world is available here.
use super::{contract, joined, operations, packed, trees};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, &'static str>;
type Key = (String, u64);
const PROJECT: &str = "10000000-0000-4000-8000-000000000001";
fn ensure(v: bool, e: &'static str) -> Result<()> {
    if v { Ok(()) } else { Err(e) }
}
fn bytes(v: &Value) -> Vec<u8> {
    photara_core::canonical_json(v).unwrap()
}
fn number(v: &Value) -> Result<u64> {
    let s = v.as_str().ok_or("route decimal")?;
    let n = s.parse::<u64>().map_err(|_| "route decimal")?;
    ensure(n.to_string() == s, "route canonical decimal")?;
    Ok(n)
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or("route string")
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("route array")
}
fn reference(v: &Value) -> Value {
    json!({"kind":"json","sha256":packed::hash(&bytes(v)),"byte_length":bytes(v).len().to_string()})
}
fn key(v: &Value) -> Result<Key> {
    trees::key(v)
}
fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let o = v.as_object().ok_or("route object")?;
    ensure(
        o.len() == names.len() && names.iter().all(|n| o.contains_key(*n)),
        "route exact fields",
    )
}
fn schema(v: &Value, name: &str, names: &[&str]) -> Result<()> {
    let mut all = vec!["schema", "project_id", "extensions"];
    all.extend_from_slice(names);
    fields(v, &all)?;
    ensure(
        v["schema"] == json!({"id":format!("example.ps2.{name}"),"version":1})
            && v["project_id"] == PROJECT,
        "route schema",
    )?;
    ensure(
        v["extensions"].as_object().is_some_and(|o| {
            o.keys()
                .all(|k| photara_core::contracts::schema::QualifiedName::parse(k.clone()).is_ok())
        }),
        "route extensions",
    )
}
pub(super) fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../../docs/architecture/proposals/ps2/route/linked-route.json"
    ))
    .unwrap()
}
pub(super) fn world(c: &Value, name: &str) -> Result<joined::World> {
    let mut v = c["snapshots"][name].clone();
    let mut files = serde_json::Map::new();
    for (id, h) in v["allocations"].as_object().ok_or("snapshot allocations")? {
        files.insert(id.clone(), c["allocation_blobs"][text(h)?].clone());
    }
    v["allocations"] = Value::Object(files);
    joined::World::load(&v)
}
fn old_ops() -> Value {
    serde_json::from_str(include_str!(
        "../../../../docs/architecture/proposals/ps2/operations/linked-operations.json"
    ))
    .unwrap()
}
fn profile() -> Value {
    serde_json::from_str::<Value>(include_str!(
        "../../../../docs/architecture/proposals/ps2/resource-conversion/linked.json"
    ))
    .unwrap()["scenarios"]["valid"]["fixture_profile"]
        .clone()
}
fn operation_store(four: bool) -> Result<operations::Store> {
    let c = if four { super::corpus() } else { old_ops() };
    let mut s = operations::Store::load(&c)?;
    if four {
        s.journal
            .push(c["records"]["journal-frame-4"]["input"].clone());
    }
    Ok(s)
}
#[derive(Clone)]
pub(super) struct Proof {
    pub(super) original: Value,
    pub(super) phase: Value,
    pub(super) ledger: Value,
    pub(super) total: u64,
}
#[allow(
    clippy::too_many_lines,
    reason = "Exact outer dispatch and selected root fields are checked together"
)]
fn bootstrap(w: &joined::World) -> Result<&Value> {
    let manifest = &old_ops()["records"]["manifest"]["input"];
    ensure(w.manifest == *manifest, "route original manifest")?;
    fields(
        &w.head,
        &["schema", "project_id", "commit_id", "commit_sha256"],
    )?;
    ensure(
        w.head["schema"] == json!({"id":"photara.package.head","version":1})
            && w.head["project_id"] == PROJECT
            && w.head["commit_id"] == w.commit["commit_id"]
            && w.head["commit_sha256"] == packed::hash(&bytes(&w.commit)),
        "route selected HEAD",
    )?;
    fields(
        &w.commit,
        &[
            "schema",
            "project_id",
            "commit_id",
            "package_revision",
            "bootstrap_sha256",
            "parent",
            "write_id",
            "created_at",
            "minimum_reader",
            "required_features",
            "authored",
            "history",
            "inventory",
            "root_set",
            "extensions",
        ],
    )?;
    ensure(
        w.commit["schema"] == json!({"id":"photara.package.commit","version":1})
            && w.commit["project_id"] == PROJECT
            && w.commit["bootstrap_sha256"] == packed::hash(&bytes(manifest))
            && w.commit["minimum_reader"] == json!({"major":1,"minor":3}),
        "route outer dispatch",
    )?;
    let mut features = array(&manifest["required_features"])?.clone();
    for s in [
        "photara.sealed-roots.v1",
        "photara.resource-backings.v1",
        "example.ps2.scalable-storage.draft-v1",
        "example.ps2.typed-branches.draft-v1",
        "example.ps2.owned-accounting.draft-v1",
        "example.ps2.selected-route.draft-v1",
        "example.ps2.inventory-overlay.draft-v1",
    ] {
        features.push(json!(s));
    }
    features.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    ensure(
        w.commit["required_features"] == json!(features),
        "route required capabilities",
    )?;
    photara_core::context::value::Timestamp::try_from(text(&w.commit["created_at"])?.to_owned())
        .map_err(|_| "route timestamp")?;
    for value in [&w.commit["commit_id"], &w.commit["write_id"]] {
        let s = text(value)?;
        let u = uuid::Uuid::parse_str(s).map_err(|_| "route UUID")?;
        ensure(!u.is_nil() && u.to_string() == s, "route canonical UUID")?;
    }
    number(&w.commit["package_revision"])?;
    fields(&w.commit["parent"], &["commit_id", "sha256"])?;
    let parent_id = text(&w.commit["parent"]["commit_id"])?;
    let id = uuid::Uuid::parse_str(parent_id).map_err(|_| "route parent UUID")?;
    ensure(
        !id.is_nil() && id.to_string() == parent_id,
        "route parent UUID",
    )?;
    let digest = text(&w.commit["parent"]["sha256"])?;
    ensure(
        digest.len() == 64
            && digest
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "route parent digest",
    )?;
    ensure(
        w.commit["extensions"].as_object().is_some_and(|o| {
            o.keys()
                .all(|k| photara_core::contracts::schema::QualifiedName::parse(k.clone()).is_ok())
        }),
        "route commit extensions",
    )?;
    let root = &w.commit["root_set"];
    schema(
        root,
        "root-set",
        &[
            "library_id",
            "bootstrap_sha256",
            "kind",
            "active",
            "recovery",
            "pinned_roots",
            "operation_index",
            "conversion_source",
            "inventory",
            "placement",
        ],
    )?;
    ensure(
        root["library_id"] == "10000000-0000-4000-8000-000000000002"
            && root["bootstrap_sha256"] == w.commit["bootstrap_sha256"]
            && root["kind"] == "scalable-draft"
            && root["pinned_roots"] == json!([]),
        "route root identity/pins",
    )?;
    fields(
        &root["placement"],
        &[
            "generation",
            "active_locator",
            "recovery_locator",
            "active_ownership",
            "recovery_ownership",
            "accounting",
        ],
    )?;
    Ok(root)
}
fn source_proof(o: &Value) -> Result<BTreeMap<String, Value>> {
    let s = &o["recipe"]["source"];
    if s.is_null() {
        return Ok(BTreeMap::new());
    }
    let mut objects = BTreeMap::new();
    for v in array(&s["charge_path"])? {
        ensure(
            objects
                .insert(key(&reference(v))?, (v.clone(), 2))
                .is_none(),
            "duplicate source proof node",
        )?;
    }
    let store = trees::Store {
        selector: Value::Null,
        objects,
    };
    let proof = trees::tree(
        &store,
        &o["old"]["ledger"]["sealed_charge_tree"],
        trees::Kind::Charge,
    )?;
    let row = proof
        .entries
        .iter()
        .find(|v| v["allocation_id"] == s["allocation_id"])
        .ok_or("original source charge inclusion")?;
    ensure(
        row["charge"] == reference(&s["charge"])
            && row["charged_high_water"] == s["registered_charge"]
            && s["charge"]["allocation_id"] == s["allocation_id"]
            && s["charge"]["measured_extent"] == s["byte_length"]
            && s["charge"]["charged_high_water"] == s["registered_charge"],
        "original source exact charge",
    )?;
    let observation = array(&o["old"]["ledger"]["sealed_observations"])?
        .iter()
        .find(|v| v["observation_id"] == s["charge"]["observation"]["observation_id"])
        .ok_or("original source observation")?;
    ensure(
        observation["witness"] == s["witness"],
        "original source immutable witness",
    )?;
    let mut expected = proof.nodes;
    let mut charges = BTreeMap::new();
    for entry in proof.entries {
        let k = key(&entry["charge"])?;
        let (v, _) = store.objects.get(&k).ok_or("source proof charge absent")?;
        charges.insert(text(&entry["allocation_id"])?.to_owned(), v.clone());
        expected.insert(k);
    }
    ensure(
        expected == store.objects.keys().cloned().collect(),
        "exact original source proof objects",
    )?;
    Ok(charges)
}
fn physical_recipe(w: &joined::World, o: &Value, admitted: bool) -> Result<()> {
    for a in array(&o["recipe"]["allocations"])? {
        let f = w
            .allocations
            .get(text(&a["allocation_id"])?)
            .ok_or("recipe allocation absent")?;
        let before = usize::try_from(number(&a["old_end"])?).map_err(|_| "recipe range")?;
        let end = usize::try_from(number(&a["end"])?).map_err(|_| "recipe range")?;
        ensure(
            f.witness == a["witness"]
                && f.arena == a["arena"]
                && f.bytes.len() == if admitted { before } else { end },
            "original recipe inode/range",
        )?;
        ensure(
            packed::hash(&f.bytes[..before]) == a["old_sha256"],
            "original recipe prefix",
        )?;
        if !admitted {
            ensure(
                packed::hash(&f.bytes) == a["sha256"]
                    && packed::hash(&f.bytes[before..]) == a["append_sha256"],
                "complete original suffix",
            )?;
            ensure(
                packed::frame_ranges(&f.bytes[before..], &f.arena)?.len() as u64
                    == number(&a["frame_count"])?,
                "exact suffix frames",
            )?;
        }
    }
    Ok(())
}
#[allow(
    clippy::too_many_lines,
    reason = "One selected authority proof ties controls, immutable recipe, semantic closure and physical accounting"
)]
pub(super) fn verify(world: &joined::World) -> Result<Proof> {
    let root = bootstrap(world)?;
    let envelope = world.loose(&root["placement"]["accounting"])?;
    schema(
        &envelope,
        "route-accounting",
        &[
            "ledger",
            "original",
            "phase",
            "project_limit",
            "reserve",
            "consumed",
            "remaining",
        ],
    )?;
    let original = world.loose(&envelope["original"])?;
    let phase = world.loose(&envelope["phase"])?;
    contract::validate_original(&original)?;
    contract::validate_phase(&original, &phase)?;
    super::control::preflight(world, std::slice::from_ref(world), &original)?;
    let admitted = phase["stage"] == "admitted";
    let retire = original["kind"] == "retire";
    let clean = phase["stage"] == "clean";
    for field in ["reserve", "project_limit"] {
        ensure(
            envelope[field] == original[field],
            "selected immutable budget",
        )?;
    }
    for field in ["consumed", "remaining"] {
        ensure(envelope[field] == phase[field], "selected phase budget")?;
    }
    let overlay = world.loose(&root["inventory"])?;
    schema(
        &overlay,
        "inventory-overlay",
        &["base", "controls", "count"],
    )?;
    let oldroot = &original["old"]["commit"]["root_set"];
    for field in if retire {
        vec!["active", "recovery", "operation_index", "conversion_source"]
    } else {
        vec!["recovery", "conversion_source"]
    } {
        ensure(
            original["target"][field] == oldroot[field],
            "original relocation and recovery semantics",
        )?;
    }
    let mut expected = if admitted {
        oldroot.clone()
    } else {
        let mut r = oldroot.clone();
        for field in ["active", "recovery", "operation_index", "conversion_source"] {
            r[field] = original["target"][field].clone();
        }
        r["placement"] = original["target"]["placement"].clone();
        r
    };
    expected["placement"]["accounting"] = root["placement"]["accounting"].clone();
    expected["inventory"] = root["inventory"].clone();
    ensure(*root == expected, "exact original target selection")?;
    let expected_base = if admitted {
        if original["old"]["overlay"].is_null() {
            &oldroot["inventory"]
        } else {
            &original["old"]["overlay"]["base"]
        }
    } else {
        &original["target"]["base_inventory"]
    };
    ensure(overlay["base"] == *expected_base, "original base inventory")?;
    let mut old_charges = source_proof(&original)?;
    physical_recipe(world, &original, admitted)?;
    let source = &original["recipe"]["source"];
    let absent = retire && matches!(text(&phase["stage"])?, "unlinked" | "clean");
    if retire {
        let file = world.allocations.get(text(&source["allocation_id"])?);
        if absent {
            ensure(file.is_none(), "authorized source absence")?;
        } else {
            let f = file.ok_or("source absent before unlink")?;
            ensure(
                f.witness == source["witness"]
                    && f.bytes.len() as u64 == number(&source["byte_length"])?
                    && packed::hash(&f.bytes) == source["sha256"],
                "original source bytes/witness",
            )?;
        }
    }
    let tickets = if retire && !admitted && !clean {
        vec![
            json!({"token":original["token"],"allocation_id":source["allocation_id"],"registered_charge":source["registered_charge"]}),
        ]
    } else {
        vec![]
    };
    let resolver = joined::Resolver::new(world, &root["placement"]["active_locator"])?;
    let accounting = joined::accounting_at(
        world,
        &resolver,
        root,
        &envelope["ledger"],
        &profile(),
        &tickets,
    )?;
    if retire {
        if !admitted {
            old_charges.remove(text(&source["allocation_id"])?);
        }
        ensure(
            accounting.charges == old_charges,
            "exact immutable surviving sealed charges",
        )?;
    }
    let oldcore = &original["old"]["ledger"];
    if admitted {
        ensure(
            accounting.value == *oldcore,
            "admission preserves charged state",
        )?;
    } else {
        let observations = array(&phase["observations"])?;
        for t in array(&accounting.value["tips"])? {
            let observed = observations
                .iter()
                .find(|v| v["allocation_id"] == t["witness"]["allocation_id"])
                .ok_or("selected observed tip")?;
            ensure(
                observed["witness"] == t["witness"]
                    && observed["extent"] == t["extent"]
                    && observed["registered_charge"] == t["registered_charge"],
                "selected observed tip equality",
            )?;
        }
        let mut preserved = oldcore["sealed_observations"].clone();
        if retire {
            preserved
                .as_array_mut()
                .ok_or("old observations")?
                .retain(|v| v["witness"]["allocation_id"] != source["allocation_id"]);
        }
        ensure(
            accounting.value["sealed_observations"] == preserved,
            "preserved original sealed observations",
        )?;
        if !retire {
            ensure(
                accounting.value["sealed_charge_tree"] == oldcore["sealed_charge_tree"],
                "Graph preserves sealed charges",
            )?;
        }
        for field in [
            "profile",
            "incarnation",
            "standing_control",
            "retained_conversion",
        ] {
            ensure(
                accounting.value[field] == oldcore[field],
                "preserved original accounting",
            )?;
        }
        let target_total = number(&oldcore["total_charge"])?
            .checked_add(number(&phase["consumed"])?)
            .and_then(|n| {
                n.checked_sub(if retire && clean {
                    number(&source["registered_charge"]).ok()?
                } else {
                    0
                })
            })
            .ok_or("route charge overflow")?;
        ensure(
            accounting.total == target_total,
            "exact original consumed charge",
        )?;
    }
    let union = resolver.tree(&overlay["base"], trees::Kind::Inventory)?;
    let store = operation_store(retire || !admitted)?;
    let mut logical = BTreeSet::new();
    let mut used = BTreeSet::new();
    for role in ["active", "recovery"] {
        let res = joined::Resolver::new(world, &root["placement"][format!("{role}_locator")])?;
        let predecessor = if role == "active" && !retire && !admitted {
            json!({"root_sha256":oldroot["active"]["sha256"],"authored_revision":"3"})
        } else if retire {
            original["old"]["states"][role]["predecessor"].clone()
        } else {
            Value::Null
        };
        let proof = joined::verify_selected_role(
            world,
            &profile(),
            role,
            &joined::SelectedLayout {
                root,
                resolver: &res,
                accounting: &accounting,
                original: &store,
                inventory_nodes: &union.nodes,
                predecessor: &predecessor,
            },
        )?;
        logical.extend(proof.semantic);
        used.extend(proof.used);
    }
    logical.extend(accounting.global.iter().cloned());
    if admitted && original["old"]["envelope"].is_null() {
        logical.insert(key(&envelope["ledger"])?);
    }
    ensure(
        union
            .entries
            .iter()
            .map(key)
            .collect::<Result<BTreeSet<_>>>()?
            == logical,
        "exact route packed base inventory",
    )?;
    let mut controls = vec![
        envelope.clone(),
        original.clone(),
        phase.clone(),
        accounting.value.clone(),
    ];
    if !retire {
        for field in ["intent", "receipt"] {
            let r = &original["operation"][field];
            let v = if let Some((v, tag)) = resolver.objects.get(&key(r)?) {
                ensure(*tag == 1, "operation control membership")?;
                v.clone()
            } else {
                world.loose(r)?
            };
            ensure(
                v == super::corpus()["records"][format!("{field}-4")]["input"],
                "original fourth operation control",
            )?;
            controls.push(v);
        }
    }
    let mut extra = BTreeSet::new();
    for v in &controls {
        let k = key(&reference(v))?;
        if !logical.contains(&k) {
            extra.insert(k);
        }
    }
    let actual = array(&overlay["controls"])?
        .iter()
        .map(key)
        .collect::<Result<Vec<_>>>()?;
    ensure(
        actual == extra.iter().cloned().collect::<Vec<_>>()
            && number(&overlay["count"])? == (logical.len() + extra.len()) as u64,
        "exact disjoint control overlay",
    )?;
    for v in controls.iter().chain(std::iter::once(&overlay)) {
        ensure(
            world.loose(&reference(v))? == *v,
            "exact canonical loose control bytes",
        )?;
    }
    let loose = controls
        .iter()
        .map(|v| key(&reference(v)).map(|k| k.0))
        .collect::<Result<BTreeSet<_>>>()?;
    let mut expected_loose = loose;
    expected_loose.insert(key(&reference(&overlay))?.0);
    ensure(
        world.loose.keys().cloned().collect::<BTreeSet<_>>() == expected_loose,
        "exact bounded loose controls",
    )?;
    let mut allocations = used.clone();
    if retire && !admitted && !absent {
        allocations.insert(text(&source["allocation_id"])?.to_owned());
    }
    ensure(
        allocations == world.allocations.keys().cloned().collect(),
        "exact owned plus ticket allocations",
    )?;
    let mut registered = accounting.charges.keys().cloned().collect::<BTreeSet<_>>();
    for tip in array(&accounting.value["tips"])? {
        registered.insert(text(&tip["witness"]["allocation_id"])?.to_owned());
    }
    ensure(
        registered == used,
        "exact retained allocation charge coverage",
    )?;
    ensure(
        accounting
            .total
            .checked_add(number(&envelope["remaining"])?)
            .is_some_and(|n| n <= number(&original["project_limit"]).unwrap_or(0)),
        "selected finite capacity",
    )?;
    Ok(Proof {
        original,
        phase,
        ledger: accounting.value,
        total: accounting.total,
    })
}
#[test]
fn selected_route_resolves_actual_frames_and_original_tokens() {
    let c = corpus();
    for name in [
        "graph-admitted",
        "graph-published",
        "graph-clean",
        "retire-admitted",
        "retire-released",
        "retire-unlinked",
        "retire-clean",
    ] {
        let w = world(&c, name).unwrap();
        let p = verify(&w).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            p.total.to_string(),
            c["snapshots"][name]["expected"]["total_charge"]
        );
    }
}
fn selected_core(w: &joined::World) -> Result<Value> {
    let v = w.loose(&w.commit["root_set"]["placement"]["accounting"])?;
    if v["schema"]["id"] == "example.ps2.route-accounting" {
        w.loose(&v["ledger"])
    } else {
        Ok(v)
    }
}
fn transition(
    before: &joined::World,
    after: &joined::World,
    prior: Option<&Proof>,
    next: &Proof,
) -> Result<()> {
    ensure(
        after.commit["parent"]
            == json!({"commit_id":before.commit["commit_id"],"sha256":before.head["commit_sha256"]})
            && number(&after.commit["package_revision"])?
                == number(&before.commit["package_revision"])?
                    .checked_add(1)
                    .ok_or("revision overflow")?,
        "exact selected parent succession",
    )?;
    if next.phase["stage"] == "admitted" {
        if let Some(p) = prior {
            ensure(
                p.phase["stage"] == "clean" && p.original["token"] != next.original["token"],
                "new admission requires exact clean original",
            )?;
        }
        let old = &next.original["old"];
        ensure(
            old["head"] == before.head
                && old["commit"] == before.commit
                && old["ledger"] == selected_core(before)?,
            "original actual selected old base",
        )?;
        for role in ["active", "recovery"] {
            let res = joined::Resolver::new(
                before,
                &before.commit["root_set"]["placement"][format!("{role}_locator")],
            )?;
            ensure(
                old["states"][role] == res.get(&before.commit["root_set"][role], 1)?,
                "original actual StateRoot bytes",
            )?;
        }
        if prior.is_some() {
            ensure(
                old["envelope"]
                    == before.loose(&before.commit["root_set"]["placement"]["accounting"])?
                    && old["overlay"] == before.loose(&before.commit["root_set"]["inventory"])?,
                "original actual previous controls",
            )?;
        }
        ensure(
            before.allocations.keys().collect::<Vec<_>>()
                == after.allocations.keys().collect::<Vec<_>>(),
            "admission allocation effects",
        )?;
        for (id, a) in &before.allocations {
            let b = &after.allocations[id];
            ensure(
                a.bytes == b.bytes && a.witness == b.witness && a.arena == b.arena,
                "admission payload effects",
            )?;
        }
    } else {
        let p = prior.ok_or("missing original admission")?;
        ensure(
            p.original == next.original,
            "immutable original across phases",
        )?;
        contract::validate_progress(
            &next.original,
            &p.phase,
            &next.phase,
            &p.ledger,
            &next.ledger,
        )?;
    }
    ensure(
        before.source == after.source && before.source_witnesses == after.source_witnesses,
        "retained conversion unchanged",
    )
}
#[test]
fn actual_old_head_admission_publication_release_and_credit_form_one_authority_chain() {
    let c = corpus();
    let mut old = world(&c, "old").unwrap();
    assert_eq!(
        joined::verify(&old, &old_ops(), &profile()).unwrap(),
        761_856
    );
    let mut prior = None;
    for name in [
        "graph-admitted",
        "graph-published",
        "graph-clean",
        "retire-admitted",
        "retire-released",
        "retire-unlinked",
        "retire-clean",
    ] {
        let current = world(&c, name).unwrap();
        let proof = verify(&current).unwrap_or_else(|e| panic!("{name}: {e}"));
        transition(&old, &current, prior.as_ref(), &proof)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        prior = Some(proof);
        old = current;
    }
    assert_eq!(prior.unwrap().total, 1_806_336);
}
fn replace(v: &mut Value, old: &Value, new: &Value) {
    if v == old {
        *v = new.clone();
        return;
    }
    match v {
        Value::Object(m) => {
            for value in m.values_mut() {
                replace(value, old, new);
            }
        }
        Value::Array(a) => {
            for value in a {
                replace(value, old, new);
            }
        }
        _ => {}
    }
}
fn edit_control(w: &mut joined::World, schema_name: &str, change: impl FnOnce(&mut Value)) {
    let (hash, raw) = w
        .loose
        .iter()
        .find(|(_, raw)| {
            serde_json::from_slice::<Value>(raw).unwrap()["schema"]["id"] == schema_name
        })
        .unwrap();
    let old: Value = serde_json::from_slice(raw).unwrap();
    let hash = hash.clone();
    let mut value = old.clone();
    change(&mut value);
    let mut pending = vec![(reference(&old), reference(&value))];
    w.loose.remove(&hash);
    w.loose.insert(
        reference(&value)["sha256"].as_str().unwrap().into(),
        bytes(&value),
    );
    loop {
        let mut changed = false;
        let before = w.commit.clone();
        for (old, new) in &pending {
            replace(&mut w.commit, old, new);
        }
        changed |= before != w.commit;
        let entries = w.loose.clone();
        let mut discovered = Vec::new();
        for (hash, raw) in entries {
            let old: Value = serde_json::from_slice(&raw).unwrap();
            let mut next = old.clone();
            for (from, to) in &pending {
                replace(&mut next, from, to);
            }
            if next != old {
                changed = true;
                w.loose.remove(&hash);
                w.loose.insert(
                    reference(&next)["sha256"].as_str().unwrap().into(),
                    bytes(&next),
                );
                discovered.push((reference(&old), reference(&next)));
            }
        }
        pending.extend(discovered);
        if !changed {
            break;
        }
    }
    w.rehash_head();
}
#[test]
fn selected_controls_reject_coherent_underreserve_unknown_refs_and_missing_capabilities() {
    let c = corpus();
    let mut w = world(&c, "graph-admitted").unwrap();
    edit_control(&mut w, "example.ps2.route-original", |o| {
        o["reserve"] = json!("1");
    });
    assert_eq!(
        verify(&w).err(),
        Some("contract original finite control/reserve")
    );
    let mut w = world(&c, "graph-published").unwrap();
    edit_control(&mut w, "example.ps2.inventory-overlay", |o| {
        o["controls"][0]["extra"] = json!(true);
    });
    assert_eq!(verify(&w).err(), Some("exact typed fields"));
    let mut w = world(&c, "graph-published").unwrap();
    w.commit["required_features"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v != "example.ps2.selected-route.draft-v1");
    w.rehash_head();
    assert_eq!(verify(&w).err(), Some("route required capabilities"));
    let mut w = world(&c, "graph-published").unwrap();
    w.commit["root_set"]["pinned_roots"] = json!([w.commit["root_set"]["active"]]);
    w.rehash_head();
    assert_eq!(verify(&w).err(), Some("route root identity/pins"));
}
#[test]
fn postunlink_proof_uses_actual_destination_and_canonical_duplicate_controls() {
    let c = corpus();
    let mut w = world(&c, "retire-clean").unwrap();
    assert!(
        !w.allocations
            .contains_key("80000000-0000-4000-8000-000000000001")
    );
    verify(&w).unwrap();
    let a = w
        .allocations
        .get_mut("80000000-0000-4000-8000-000000000004")
        .unwrap();
    let end = a.bytes.len() - 1;
    a.bytes[end] = 1;
    assert_eq!(verify(&w).err(), Some("complete original suffix"));
    let mut w = world(&c, "graph-published").unwrap();
    let receipt = super::corpus()["records"]["receipt-4"]["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    w.loose.get_mut(&receipt).unwrap()[0] = b'!';
    assert!(verify(&w).is_err());
    let mut w = world(&c, "retire-clean").unwrap();
    w.allocations
        .get_mut("80000000-0000-4000-8000-000000000004")
        .unwrap()
        .witness["inode"] = json!("999");
    assert_eq!(verify(&w).err(), Some("original recipe inode/range"));
}
/// Read-only role open: independently proves recovery bytes and its own physical
/// closure. It does not certify absent active tips for writable admission.
#[allow(
    clippy::too_many_lines,
    reason = "Independent selected recovery controls and physical closure are checked without active reads"
)]
fn recovery(w: &joined::World) -> Result<joined::RoleProof> {
    let root = bootstrap(w)?;
    let envelope = w.loose(&root["placement"]["accounting"])?;
    schema(
        &envelope,
        "route-accounting",
        &[
            "ledger",
            "original",
            "phase",
            "project_limit",
            "reserve",
            "consumed",
            "remaining",
        ],
    )?;
    let o = w.loose(&envelope["original"])?;
    let p = w.loose(&envelope["phase"])?;
    contract::validate_original(&o)?;
    contract::validate_phase(&o, &p)?;
    super::control::preflight(w, std::slice::from_ref(w), &o)?;
    let admitted = p["stage"] == "admitted";
    let retire = o["kind"] == "retire";
    let original_root = &o["old"]["commit"]["root_set"];
    ensure(
        root["recovery"] == original_root["recovery"]
            && o["target"]["recovery"] == original_root["recovery"],
        "independent original recovery selection",
    )?;
    let selected = if admitted {
        &original_root["placement"]
    } else {
        &o["target"]["placement"]
    };
    for field in ["recovery_locator", "recovery_ownership"] {
        ensure(
            root["placement"][field] == selected[field],
            "independent original recovery placement",
        )?;
    }
    for field in ["reserve", "project_limit"] {
        ensure(envelope[field] == o[field], "selected immutable budget")?;
    }
    for field in ["consumed", "remaining"] {
        ensure(envelope[field] == p[field], "selected phase budget")?;
    }
    let overlay = w.loose(&root["inventory"])?;
    schema(
        &overlay,
        "inventory-overlay",
        &["base", "controls", "count"],
    )?;
    for r in array(&overlay["controls"])? {
        key(r)?;
        w.loose(r)?;
    }
    let base = if admitted {
        if o["old"]["overlay"].is_null() {
            &original_root["inventory"]
        } else {
            &o["old"]["overlay"]["base"]
        }
    } else {
        &o["target"]["base_inventory"]
    };
    ensure(overlay["base"] == *base, "independent original inventory")?;
    let resolver = joined::Resolver::new(w, &root["placement"]["recovery_locator"])?;
    let source = &o["recipe"]["source"];
    let tickets = if retire && !admitted && p["stage"] != "clean" {
        vec![
            json!({"token":o["token"],"allocation_id":source["allocation_id"],"registered_charge":source["registered_charge"]}),
        ]
    } else {
        vec![]
    };
    let account = joined::accounting_at(
        w,
        &resolver,
        root,
        &envelope["ledger"],
        &profile(),
        &tickets,
    )?;
    let union = resolver.tree(&overlay["base"], trees::Kind::Inventory)?;
    let base_keys = union
        .entries
        .iter()
        .map(key)
        .collect::<Result<BTreeSet<_>>>()?;
    let mut controls = vec![
        envelope.clone(),
        o.clone(),
        p.clone(),
        account.value.clone(),
    ];
    if !retire {
        for field in ["intent", "receipt"] {
            let v = w.loose(&o["operation"][field])?;
            ensure(
                v == super::corpus()["records"][format!("{field}-4")]["input"],
                "original fourth operation control",
            )?;
            controls.push(v);
        }
    }
    let mut expected = BTreeSet::new();
    for v in &controls {
        let k = key(&reference(v))?;
        if !base_keys.contains(&k) {
            expected.insert(k);
        }
    }
    let actual = array(&overlay["controls"])?
        .iter()
        .map(key)
        .collect::<Result<Vec<_>>>()?;
    ensure(
        actual == expected.iter().cloned().collect::<Vec<_>>()
            && number(&overlay["count"])? == (base_keys.len() + expected.len()) as u64,
        "independent exact control overlay",
    )?;
    let mut loose = BTreeSet::new();
    for v in controls.iter().chain(std::iter::once(&overlay)) {
        w.loose(&reference(v))?;
        loose.insert(key(&reference(v))?.0);
    }
    ensure(
        loose == w.loose.keys().cloned().collect(),
        "independent exact loose controls",
    )?;
    if retire {
        let mut charges = source_proof(&o)?;
        if !admitted {
            charges.remove(text(&source["allocation_id"])?);
        }
        ensure(
            charges == account.charges,
            "exact immutable surviving sealed charges",
        )?;
    }
    if admitted {
        ensure(
            account.value == o["old"]["ledger"],
            "admission preserves charged state",
        )?;
    } else {
        for field in [
            "profile",
            "incarnation",
            "standing_control",
            "retained_conversion",
        ] {
            ensure(
                account.value[field] == o["old"]["ledger"][field],
                "preserved original accounting",
            )?;
        }
        for tip in array(&account.value["tips"])? {
            let observed = array(&p["observations"])?
                .iter()
                .find(|r| r["allocation_id"] == tip["witness"]["allocation_id"])
                .ok_or("selected observed tip")?;
            ensure(
                observed["witness"] == tip["witness"]
                    && observed["extent"] == tip["extent"]
                    && observed["registered_charge"] == tip["registered_charge"],
                "selected observed tip equality",
            )?;
        }
        let total = number(&o["old"]["ledger"]["total_charge"])?
            .checked_add(number(&p["consumed"])?)
            .and_then(|n| {
                n.checked_sub(if retire && p["stage"] == "clean" {
                    number(&source["registered_charge"]).ok()?
                } else {
                    0
                })
            })
            .ok_or("route charge overflow")?;
        ensure(account.total == total, "exact original consumed charge")?;
    }
    let mut retained = o.clone();
    retained["recipe"]["allocations"]
        .as_array_mut()
        .ok_or("recipe")?
        .retain(|a| {
            a["allocation_id"]
                .as_str()
                .is_some_and(|id| w.allocations.contains_key(id))
        });
    physical_recipe(w, &retained, admitted)?;
    let predecessor = &o["old"]["states"]["recovery"]["predecessor"];
    joined::verify_selected_role(
        w,
        &profile(),
        "recovery",
        &joined::SelectedLayout {
            root,
            resolver: &resolver,
            accounting: &account,
            original: &operation_store(retire || !admitted)?,
            inventory_nodes: &union.nodes,
            predecessor,
        },
    )
}
#[test]
fn new_route_recovery_is_independent_of_absent_active_allocations() {
    let c = corpus();
    for name in ["graph-published", "retire-clean"] {
        let mut w = world(&c, name).unwrap();
        let proof = recovery(&w).unwrap();
        w.allocations.retain(|id, _| proof.used.contains(id));
        let independently = recovery(&w).unwrap();
        assert_eq!(independently.state["accepted"]["through_ordinal"], "2");
        assert!(verify(&w).is_err());
    }
}
fn original_receipt(w: &joined::World, intent: &Value) -> Result<Value> {
    verify(w)?;
    let root = &w.commit["root_set"];
    let res = joined::Resolver::new(w, &root["placement"]["active_locator"])?;
    let index = res.get(&root["operation_index"], 1)?;
    let tree = res.tree(&index["by_id"], trees::Kind::Id)?;
    let entry = tree
        .entries
        .iter()
        .find(|e| e["operation_id"] == intent["operation_id"])
        .ok_or("operation absent")?;
    ensure(
        entry["request_sha256"] == packed::hash(&bytes(intent)),
        "same operation ID changed request",
    )?;
    res.get(&entry["receipt"], 1)
}
fn cleanup_retry(w: &joined::World, original: &Value) -> Result<Value> {
    let proof = verify(w)?;
    ensure(
        proof.phase["stage"] == "clean" && reference(&proof.original) == *original,
        "exact completed original retry",
    )?;
    Ok(
        json!({"token":proof.original["token"],"consumed":proof.phase["consumed"],"credit":proof.phase["credit"],"total_charge":proof.ledger["total_charge"],"additional_credit":"0"}),
    )
}
#[test]
fn terminal_original_retry_and_receipt_provenance_survive_source_removal() {
    let c = corpus();
    let w = world(&c, "retire-clean").unwrap();
    let proof = verify(&w).unwrap();
    let original = reference(&proof.original);
    let first = cleanup_retry(&w, &original).unwrap();
    assert_eq!(first["credit"], "4096");
    assert_eq!(first["additional_credit"], "0");
    assert_eq!(cleanup_retry(&w, &original).unwrap(), first);
    assert_eq!(
        cleanup_retry(
            &w,
            &json!({"kind":"json","sha256":"f".repeat(64),"byte_length":"1"})
        )
        .err(),
        Some("exact completed original retry")
    );
    let ops = super::corpus();
    for n in 1..=4 {
        let intent = &ops["records"][format!("intent-{n}")]["input"];
        assert_eq!(
            original_receipt(&w, intent).unwrap(),
            ops["records"][format!("receipt-{n}")]["input"]
        );
    }
    let mut altered = ops["records"]["intent-1"]["input"].clone();
    altered["command"]["envelope"]["command"]["x"] = json!(101);
    assert_eq!(
        original_receipt(&w, &altered).err(),
        Some("same operation ID changed request")
    );
    let mut forged = w;
    edit_control(&mut forged, "example.ps2.route-phase", |p| {
        p["credit"] = json!("8192");
    });
    assert!(cleanup_retry(&forged, &original).is_err());
}
#[test]
fn coherent_retirement_state_and_surviving_charge_changes_refuse() {
    let negatives: Value = serde_json::from_str(include_str!(
        "../../../../docs/architecture/proposals/ps2/route/route-negatives.json"
    ))
    .unwrap();
    for (name, error) in [
        (
            "changed-retirement-state",
            "original relocation and recovery semantics",
        ),
        (
            "changed-survivor-charge",
            "exact immutable surviving sealed charges",
        ),
    ] {
        let mut c = corpus();
        c["allocation_blobs"].as_object_mut().unwrap().extend(
            negatives[name]["allocation_blobs"]
                .as_object()
                .unwrap()
                .clone(),
        );
        c["snapshots"]["negative"] = negatives[name]["snapshot"].clone();
        assert_eq!(
            verify(&world(&c, "negative").unwrap()).err(),
            Some(error),
            "{name}"
        );
    }
}
#[test]
fn independent_recovery_requires_the_actual_selected_control_closure() {
    let c = corpus();
    let mut w = world(&c, "retire-clean").unwrap();
    edit_control(&mut w, "example.ps2.inventory-overlay", |v| {
        v["controls"].as_array_mut().unwrap().pop();
    });
    assert_eq!(
        recovery(&w).err(),
        Some("independent exact control overlay")
    );
}
#[test]
fn actual_source_witness_and_original_charge_proof_refuse_substitution() {
    let c = corpus();
    let mut w = world(&c, "retire-released").unwrap();
    w.allocations
        .get_mut("80000000-0000-4000-8000-000000000001")
        .unwrap()
        .witness["inode"] = json!("999");
    assert_eq!(verify(&w).err(), Some("original source bytes/witness"));
    let mut w = world(&c, "retire-clean").unwrap();
    edit_control(&mut w, "example.ps2.route-original", |v| {
        v["recipe"]["source"]["charge"]["charged_high_water"] = json!("8192");
    });
    assert_eq!(verify(&w).err(), Some("original source exact charge"));
}
#[test]
fn complete_future_control_and_payload_plan_is_admitted_before_any_effect() {
    let c = corpus();
    for (old_name, names) in [
        (
            "old",
            vec!["graph-admitted", "graph-published", "graph-clean"],
        ),
        (
            "graph-clean",
            vec![
                "retire-admitted",
                "retire-released",
                "retire-unlinked",
                "retire-clean",
            ],
        ),
    ] {
        let old = world(&c, old_name).unwrap();
        let candidates = names
            .iter()
            .map(|name| world(&c, name).unwrap())
            .collect::<Vec<_>>();
        let original =
            preflight_admission(&old, &candidates).unwrap_or_else(|e| panic!("{old_name}: {e}"));
        let before = old.head.clone();
        let mut refused = original.clone();
        refused["maximum_control_bytes"] = json!("4096");
        assert!(super::control::preflight(&old, &candidates, &refused).is_err());
        assert_eq!(old.head, before);
    }
}
fn selector_intent(before: &joined::World, after: &joined::World, original: &Value) -> Value {
    json!({"schema":{"id":"example.ps2.route-selector-intent","version":1},"project_id":PROJECT,"extensions":{},"token":original["token"],"original":reference(original),"current_head_sha256":packed::hash(&bytes(&before.head)),"next_head_sha256":packed::hash(&bytes(&after.head))})
}
fn preflight_admission(old: &joined::World, candidates: &[joined::World]) -> Result<Value> {
    let first = candidates
        .first()
        .ok_or("complete original phase coverage")?;
    let envelope = first.loose(&first.commit["root_set"]["placement"]["accounting"])?;
    let original = first.loose(&envelope["original"])?;
    let stages = if original["kind"] == "graph" {
        vec!["admitted", "published", "clean"]
    } else {
        vec!["admitted", "released", "unlinked", "clean"]
    };
    ensure(
        candidates.len() == stages.len(),
        "complete original phase coverage",
    )?;
    for (world, stage) in candidates.iter().zip(stages) {
        let e = world.loose(&world.commit["root_set"]["placement"]["accounting"])?;
        ensure(
            e["original"] == reference(&original),
            "preflight same immutable original",
        )?;
        let p = world.loose(&e["phase"])?;
        ensure(p["stage"] == stage, "complete original phase coverage")?;
    }
    let mut prior = if old.commit["required_features"]
        .as_array()
        .ok_or("features")?
        .contains(&json!("example.ps2.selected-route.draft-v1"))
    {
        Some(verify(old)?)
    } else {
        joined::verify(old, &old_ops(), &profile())?;
        None
    };
    let mut previous = old;
    for world in candidates {
        let proof = verify(world)?;
        transition(previous, world, prior.as_ref(), &proof)?;
        ensure(
            bytes(&selector_intent(previous, world, &original)).len() <= 4096,
            "actual selector intent slot",
        )?;
        previous = world;
        prior = Some(proof);
    }
    super::control::preflight(old, candidates, &original)?;
    Ok(original)
}
#[test]
fn admission_requires_all_same_original_phases_and_exact_small_intent_bytes() {
    let c = corpus();
    let old = world(&c, "old").unwrap();
    let mut candidates = ["graph-admitted", "graph-published", "graph-clean"]
        .iter()
        .map(|n| world(&c, n).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        preflight_admission(&old, &candidates[..2]).err(),
        Some("complete original phase coverage")
    );
    let envelope = candidates[0]
        .loose(&candidates[0].commit["root_set"]["placement"]["accounting"])
        .unwrap();
    let original = candidates[0].loose(&envelope["original"]).unwrap();
    let intent = selector_intent(&old, &candidates[0], &original);
    assert!(bytes(&intent).len() <= 4096);
    assert_eq!(
        bytes(&intent),
        c["transient_intents"]["graph-admitted"]["canonical"]
            .as_str()
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        packed::hash(&bytes(&intent)),
        c["transient_intents"]["graph-admitted"]["sha256"]
    );
    let mut previous = old.clone();
    for name in array(&c["order"]).unwrap().iter().skip(1) {
        let current = world(&c, name.as_str().unwrap()).unwrap();
        let e = current
            .loose(&current.commit["root_set"]["placement"]["accounting"])
            .unwrap();
        let o = current.loose(&e["original"]).unwrap();
        let value = selector_intent(&previous, &current, &o);
        let fixed = &c["transient_intents"][name.as_str().unwrap()];
        assert_eq!(
            bytes(&value),
            fixed["canonical"].as_str().unwrap().as_bytes()
        );
        assert_eq!(packed::hash(&bytes(&value)), fixed["sha256"]);
        assert!(bytes(&value).len() <= 4096);
        let (required, roles) = super::control::peak(&previous, &current).unwrap();
        assert_eq!(
            json!({"rounded_bytes":required,"roles":roles}),
            c["control_peaks"][name.as_str().unwrap()]
        );
        previous = current;
    }
    edit_control(&mut candidates[2], "example.ps2.route-original", |o| {
        o["token"] = json!("303");
    });
    assert_eq!(
        preflight_admission(&old, &candidates).err(),
        Some("preflight same immutable original")
    );
}
#[test]
fn selected_and_independent_recovery_refuse_oversized_current_controls() {
    let c = corpus();
    let mut w = world(&c, "retire-clean").unwrap();
    edit_control(&mut w, "example.ps2.route-accounting", |v| {
        v["extensions"]["example.large"] = json!("x".repeat(131_072));
    });
    assert_eq!(
        verify(&w).err(),
        Some("control simultaneous rounded byte cap")
    );
    assert_eq!(
        recovery(&w).err(),
        Some("control simultaneous rounded byte cap")
    );
}
