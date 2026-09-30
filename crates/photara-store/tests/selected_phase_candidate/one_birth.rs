//! One real receipt frame born and sealed; all finalization remains in original tips.
//! Pure selected bytes/observations, not native create/link/barrier qualification.
use super::{decoder, phases, prepare, replay_prepare, wire};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use wire::{Result, encode, ensure, hash, key, number, reference, text};
const CODEC: &str = "photara.codec.ps2-single-birth-layout-v1";
const STAGES: [&str; 10] = [
    "birth-intended",
    "birth-bound",
    "birth-empty",
    "birth-promoted",
    "journal-durable",
    "payload-durable",
    "finalizer-selected",
    "finalizer-durable",
    "published",
    "clean",
];
fn id(n: u64) -> String {
    format!("96000000-0000-4000-8000-{n:012}")
}
fn born_id() -> String {
    id(1009)
}
fn obj(name: &str, mut v: Value) -> Value {
    v["schema"] = json!({"id":format!("photara.storage.{name}"),"version":1});
    v["project_id"] = json!(wire::PROJECT);
    v["extensions"] = json!({});
    v
}
fn payload(prepared: &prepare::Prepared) -> Vec<u8> {
    let bytes = encode(&prepared.receipt);
    let mut raw = b"PS2PKD01".to_vec();
    raw.extend([1, 0, 0, 0]);
    raw.extend(u32::try_from(bytes.len()).unwrap().to_le_bytes());
    raw.extend(bytes);
    raw
}
fn rounded(n: usize) -> u64 {
    (n as u64).div_ceil(4096) * 4096
}
struct Route {
    worlds: Vec<wire::World>,
    original: Value,
    plan: Value,
    finalizer: Value,
    prepared: prepare::Prepared,
    old: wire::World,
    peak: u64,
}

fn original_world(w: &wire::World, o: &Value) -> Result<wire::World> {
    let mut present = w.clone();
    present.allocations.remove(&born_id());
    phases::original_view(&present, o)
}
#[allow(
    clippy::too_many_lines,
    reason = "Exact finite original field and retained-state validation"
)]
fn validate_original(
    old: &wire::World,
    template: &Value,
    prepared: &prepare::Prepared,
) -> Result<wire::Proof> {
    let proof = wire::verify(old)?;
    wire::schema(
        template,
        "photara.storage.original-admission",
        1,
        &[
            "token",
            "kind",
            "nonce",
            "scope",
            "original_codec",
            "request",
            "old",
            "semantic_target",
            "payload_recipe",
            "old_hold",
            "generation_plan",
            "retention_intent",
            "reserve",
            "cleanup_bound",
            "project_limit",
            "control_bounds",
        ],
    )?;
    ensure(
        number(&template["token"])? > 0 && template["kind"] == "graph",
        "birth original kind/token",
    )?;
    let nonce = text(&template["nonce"])?;
    ensure(
        nonce.len() == 64
            && nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "birth original nonce",
    )?;
    wire::fields(&template["scope"], &["profile", "incarnation", "directory"])?;
    ensure(
        template["scope"]["profile"] == template["old"]["ledger"]["profile"]
            && template["scope"]["incarnation"] == template["old"]["ledger"]["incarnation"]
            && template["scope"]["directory"] == json!({"device":"7","inode":"99"}),
        "birth original local scope",
    )?;
    wire::fields(
        &template["old"],
        &["head", "commit", "envelope", "ledger", "overlay", "states"],
    )?;
    let states = template["old"]["states"]
        .as_array()
        .ok_or("birth original state array")?;
    ensure(
        states.len() == proof.roles.len(),
        "birth original exact state count",
    )?;
    let mut seen = std::collections::BTreeSet::new();
    for row in states {
        wire::fields(row, &["root", "value"])?;
        ensure(
            row["root"] == reference(&row["value"])
                && seen.insert(key(&row["root"])?)
                && proof.roles.values().any(|p| p.state == row["value"]),
            "birth original exact state body",
        )?;
    }
    wire::fields(
        &template["request"],
        &["operation_id", "request_sha256", "intent", "receipt"],
    )?;
    ensure(
        template["request"]["intent"] == reference(&prepared.intent)
            && template["request"]["receipt"] == reference(&prepared.receipt)
            && template["request"]["operation_id"] == prepared.receipt["operation_id"]
            && template["request"]["request_sha256"] == prepared.receipt["request_sha256"],
        "birth original exact request",
    )?;
    let env = old.loose(&old.commit["root_set"]["placement"]["accounting"])?;
    ensure(
        template["old_hold"] == env["holds"] && number(&template["cleanup_bound"])? == 65_536,
        "birth original old hold/cleanup",
    )?;
    wire::fields(
        &template["control_bounds"],
        &[
            "marker_bytes",
            "newborn_binding_bytes",
            "finalization_bytes",
            "phase_bytes",
            "aggregate_control_bytes",
            "aggregate_control_count",
        ],
    )?;
    ensure(
        number(&template["control_bounds"]["aggregate_control_bytes"])? <= 131_072
            && number(&template["control_bounds"]["aggregate_control_count"])? <= 24
            && template["control_bounds"]["finalization_bytes"] == "16384"
            && template["control_bounds"]["phase_bytes"] == "4096",
        "birth original control ceilings",
    )?;
    if !template["retention_intent"].is_null() {
        wire::fields(&template["retention_intent"], &["object", "body"])?;
        let intent = &template["retention_intent"]["body"];
        wire::schema(
            intent,
            "photara.resource.pending-retention-intent",
            1,
            &[
                "library_id",
                "bootstrap_sha256",
                "operation_id",
                "request_sha256",
                "requirements",
            ],
        )?;
        ensure(
            reference(intent) == template["retention_intent"]["object"]
                && intent["library_id"] == old.commit["root_set"]["library_id"]
                && intent["bootstrap_sha256"] == old.commit["bootstrap_sha256"]
                && intent["operation_id"] == prepared.receipt["operation_id"]
                && intent["request_sha256"] == prepared.receipt["request_sha256"],
            "pending original intent identity",
        )?;
        key(&intent["requirements"])?;
    }
    Ok(proof)
}
fn base(
    old: &wire::World,
    template: &Value,
    prepared: &prepare::Prepared,
) -> Result<(Value, Value)> {
    let proof = validate_original(old, template, prepared)?;
    let pending = (!template["retention_intent"].is_null()).then(|| decoder::PendingLayout {
        intent: template["retention_intent"]["body"].clone(),
        evidence: None,
    });
    let semantic = decoder::regenerate_pending(old, &proof, prepared, None, pending.as_ref())?.0;
    let raw = payload(prepared);
    let mut o = template.clone();
    o["original_codec"] = json!(if pending.is_some() {
        "photara.codec.ps2-single-birth-pending-admission-v1"
    } else {
        "photara.codec.ps2-single-birth-admission-v1"
    });
    o["semantic_target"] = semantic.target;
    o["reserve"] = json!((7 * 262_144 - 4096 + rounded(raw.len()) + 65_536).to_string());
    let mut prefixes = Vec::new();
    let mut corridors = Vec::new();
    for (aid, a) in &old.allocations {
        let sealed = aid == &id(1001);
        let end = a.bytes.len()
            + if sealed {
                0
            } else if aid == &id(1002) {
                258_048
            } else {
                262_144
            };
        prefixes.push(json!({"allocation_id":aid,"arena":a.arena,"witness":a.witness,"original_end":a.bytes.len().to_string(),"original_sha256":hash(&a.bytes),"maximum_end":end.to_string()}));
        if !sealed {
            corridors.push(json!({"allocation_id":aid,"arena":a.arena,"start":a.bytes.len().to_string(),"maximum_end":end.to_string()}));
        }
    }
    corridors.sort_by_key(|v| {
        (
            v["arena"].as_str().unwrap().to_owned(),
            v["allocation_id"].as_str().unwrap().to_owned(),
        )
    });
    o["payload_recipe"] = json!({"codec":CODEC,"allocations":prefixes,"newborn_payload":{"allocation_id":born_id(),"object":reference(&prepared.receipt),"byte_length":raw.len().to_string(),"sha256":hash(&raw)},"corridor":corridors});
    o["control_bounds"]["marker_bytes"] = json!("4096");
    o["control_bounds"]["newborn_binding_bytes"] = json!("4096");
    let slots = json!([{"ordinal":"1","allocation_id":born_id(),"arena":"data","stage_path":["birth","slot-9"],"final_path":["packs","data-9"],"maximum_extent":raw.len().to_string(),"marker_allocation_bound":"4096"}]);
    let shape = json!({"old_head_sha256":hash(&encode(&old.head)),"payload_recipe_sha256":hash(&encode(&o["payload_recipe"])),"semantic_target_sha256":hash(&encode(&o["semantic_target"])),"slots":slots,"corridor":corridors,"finalizer_codec":CODEC});
    let plan = obj(
        "generation-plan",
        json!({"token":o["token"],"nonce":o["nonce"],"scope":o["scope"],"request_sha256":o["request"]["request_sha256"],"old_head_sha256":shape["old_head_sha256"],"reserve":o["reserve"],"cleanup_bound":o["cleanup_bound"],"project_limit":o["project_limit"],"payload_codec":CODEC,"payload_recipe_sha256":shape["payload_recipe_sha256"],"semantic_target_sha256":shape["semantic_target_sha256"],"slots":slots,"finalizer_codec":CODEC,"corridor":corridors,"shape_sha256":hash(&encode(&shape)),"control_bounds":o["control_bounds"]}),
    );
    o["generation_plan"] = reference(&plan);
    ensure(
        number(&o["old"]["ledger"]["total_charge"])? + number(&o["reserve"])?
            <= number(&o["project_limit"])?,
        "birth original project admission",
    )?;
    Ok((o, plan))
}
fn marker(o: &Value, plan: &Value) -> Value {
    obj(
        "generation-marker",
        json!({"token":o["token"],"generation_plan":reference(plan),"nonce":o["nonce"],"ordinal":"1","allocation_id":born_id(),"arena":"data"}),
    )
}
fn binding(o: &Value, plan: &Value, marker: &Value, witness: &Value, stage: usize) -> Value {
    obj(
        "newborn-binding",
        json!({"token":o["token"],"generation_plan":reference(plan),"slots":[{"ordinal":"1","state":(["intended","bound","empty","promoted"][stage.min(3)]),"witness":if stage==0{Value::Null}else{json!({"device":witness["device"],"inode":witness["inode"],"marker_byte_length":encode(marker).len().to_string(),"marker_sha256":hash(&encode(marker)),"marker_observed_charge":rounded(encode(marker).len()).to_string()})}}]}),
    )
}
fn range(id: &str, a: &wire::Allocation, lo: usize) -> Result<Value> {
    let mut p = lo;
    let mut count = 0;
    while p < a.bytes.len() {
        ensure(a.bytes.len() - p >= 16, "birth full frame header")?;
        p += 16 + u32::from_le_bytes(a.bytes[p + 12..p + 16].try_into().unwrap()) as usize;
        count += 1;
    }
    ensure(p == a.bytes.len(), "birth full frame suffix")?;
    Ok(
        json!({"allocation_id":id,"arena":a.arena,"witness":a.witness,"original_end":lo.to_string(),"original_sha256":hash(&a.bytes[..lo]),"final_end":a.bytes.len().to_string(),"frame_count":count.to_string(),"framed_sha256":hash(&a.bytes[lo..])}),
    )
}
#[allow(
    clippy::too_many_lines,
    reason = "One bounded selected chain shares the existing physical decoder and reader"
)]
fn construct(
    old: wire::World,
    template: &Value,
    prepared: prepare::Prepared,
    witness: &Value,
) -> Result<Route> {
    let (o, generation) = base(&old, template, &prepared)?;
    let proof = wire::verify(&old)?;
    let pending = if o["retention_intent"].is_null() {
        None
    } else {
        let intent = o["retention_intent"]["body"].clone();
        let mut evidence = obj(
            "unused",
            json!({"library_id":old.commit["root_set"]["library_id"],"bootstrap_sha256":old.commit["bootstrap_sha256"],"operation_id":prepared.receipt["operation_id"],"request_sha256":prepared.receipt["request_sha256"],"token":o["token"],"original_admission":reference(&o),"retention_intent":reference(&intent)}),
        );
        evidence["schema"]["id"] = json!("photara.resource.pending-retention-evidence");
        Some(decoder::PendingLayout {
            intent,
            evidence: Some(evidence),
        })
    };
    let (plan, born) =
        decoder::regenerate_pending(&old, &proof, &prepared, Some(witness), pending.as_ref())?;
    let born = born.ok_or("birth attribution")?;
    let raw = payload(&prepared);
    ensure(
        plan.files[&born_id()] == raw,
        "actual newborn live receipt frame",
    )?;
    let mark = marker(&o, &generation);
    ensure(encode(&mark).len() <= 4096, "birth marker bound")?;
    let bound = binding(&o, &generation, &mark, witness, 3);
    let mut ledger = o["old"]["ledger"].clone();
    let mut growth = number(&born.charge["charged_high_water"])?;
    for t in ledger["tips"].as_array_mut().unwrap() {
        let n = plan.files[text(&t["allocation_id"])?].len();
        let charge = rounded(n);
        growth += charge - number(&t["registered_charge"])?;
        t["extent"] = json!(n.to_string());
        t["registered_charge"] = json!(charge.to_string());
        t["observation"]["measured_extent"] = json!(n.to_string());
        t["observation"]["charged_high_water"] = json!(charge.to_string());
    }
    ensure(
        growth + 65_536 == number(&o["reserve"])?,
        "birth original reserve covers exact growth",
    )?;
    ledger["sealed_charge_root"] = born.charge_root;
    ledger["observation_root"] = born.observation_root;
    ledger["total_charge"] = json!((number(&ledger["total_charge"])? + growth).to_string());
    let mut writes = Vec::new();
    for (aid, a) in &old.allocations {
        if aid == &id(1001) {
            continue;
        }
        let mut final_a = a.clone();
        final_a.bytes.clone_from(&plan.files[aid]);
        let mut r = range(aid, &final_a, a.bytes.len())?;
        r["recipe_sha256"] = json!(hash(&encode(&r)));
        writes.push(r);
    }
    let mut payload_range = range(
        &born_id(),
        &wire::Allocation {
            bytes: raw.clone(),
            arena: "data".into(),
            witness: witness.clone(),
        },
        0,
    )?;
    payload_range["framed_bytes"] = json!(raw.len().to_string());
    let active = phases::packed_value(&plan, &plan.target["active"])?;
    let mut target = plan.target.clone();
    target["base_inventory"] = plan.base_inventory.clone();
    target["placement"] = json!({"generation":"2","root_placements":plan.root_placements});
    target["retention_evidence"] = born
        .retention_evidence
        .clone()
        .unwrap_or_else(|| old.commit["root_set"]["retention_evidence"].clone());
    target["active_state"] = active.clone();
    let f = obj(
        "finalization-control",
        json!({"token":o["token"],"original":reference(&o),"prepared_receipt":prepared.receipt,"generation_plan":reference(&generation),"newborn_binding":reference(&bound),"payload_completion":[payload_range],"sealed_allocations":[{"allocation_id":born_id(),"observation":born.observation,"charge":born.charge}],"recipe_codec":CODEC,"writes":writes,"target_projection":target}),
    );
    ensure(
        encode(&f).len() as u64 <= number(&o["control_bounds"]["finalization_bytes"])?,
        "birth finalizer original bound",
    )?;
    let frame = json!({"schema":{"id":"photara.package.accepted-journal-frame","version":1},"journal_id":prepared.receipt["journal_id"],"sequence":prepared.receipt["journal_sequence"],"operation_id":prepared.receipt["operation_id"],"request_sha256":prepared.receipt["request_sha256"],"receipt_sha256":reference(&prepared.receipt)["sha256"]});
    let mut worlds = Vec::new();
    let mut prior = old.clone();
    for (stage, name) in STAGES.iter().enumerate() {
        let published = stage >= 8;
        let consumed = if published { growth } else { 0 };
        let remaining = if stage == 9 {
            0
        } else {
            number(&o["reserve"])? - consumed
        };
        let binding = binding(&o, &generation, &mark, witness, stage);
        let phase = obj(
            "operation-phase",
            json!({"token":o["token"],"original":reference(&o),"stage":name,"consumed":consumed.to_string(),"remaining":remaining.to_string(),"newborn_binding":reference(&binding),"finalization":if stage>=6{reference(&f)}else{Value::Null},"cleanup":if stage==9{json!({"remaining_roles":[],"directory_barrier":true,"released":"65536"})}else{Value::Null},"journal_completion":if stage>=4{json!({"frame":frame,"barrier":true})}else{Value::Null}}),
        );
        ensure(
            encode(&phase).len() as u64 <= number(&o["control_bounds"]["phase_bytes"])?,
            "birth phase original bound",
        )?;
        let hold = obj(
            "hold-leaf",
            json!({"count":"1","reserved":o["reserve"],"consumed":phase["consumed"],"remaining":phase["remaining"],"entries":[{"token":o["token"],"original":reference(&o),"phase":reference(&phase)}]}),
        );
        let selectedledger = if published {
            &ledger
        } else {
            &o["old"]["ledger"]
        };
        let env = obj(
            "accounting-envelope",
            json!({"ledger":reference(selectedledger),"holds":reference(&hold)}),
        );
        let mut controls = vec![
            selectedledger.clone(),
            env.clone(),
            o.clone(),
            phase,
            hold,
            prepared.intent.clone(),
            generation.clone(),
            binding,
        ];
        if !published {
            controls.push(prepared.receipt.clone());
        }
        if stage >= 6 {
            controls.push(f.clone());
        }
        let base = if published {
            plan.base_inventory.clone()
        } else {
            o["old"]["overlay"]["base"].clone()
        };
        let count = if published {
            number(&phases::packed_value(&plan, &base)?["count"])?
        } else {
            number(&o["old"]["overlay"]["count"])?
                - o["old"]["overlay"]["controls"].as_array().unwrap().len() as u64
        };
        let mut refs = controls.iter().map(reference).collect::<Vec<_>>();
        refs.sort_by_key(|r| key(r).unwrap());
        let mut overlay = obj(
            "unused",
            json!({"base":base,"controls":refs,"count":(count+controls.len() as u64).to_string()}),
        );
        overlay["schema"]["id"] = json!("photara.package.inventory-overlay");
        let mut root = old.commit["root_set"].clone();
        if published {
            for (name, v) in plan.target.as_object().unwrap() {
                root[name] = v.clone();
            }
            root["retention_evidence"] = target["retention_evidence"].clone();
            root["placement"]["generation"] = json!("2");
            root["placement"]["root_placements"] = plan.root_placements.clone();
        }
        root["inventory"] = reference(&overlay);
        root["placement"]["accounting"] = reference(&env);
        let mut next = phases::publish_control(
            &old,
            &prior,
            root,
            &controls,
            &overlay,
            9500 + stage as u64 * 2,
            if published { Some(&active) } else { None },
        )?;
        if pending.is_some() {
            let features = next.commit["required_features"]
                .as_array_mut()
                .ok_or("pending commit features")?;
            features.push(json!("photara.resource-retention-evidence.v1"));
            features.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
            features.dedup();
            next.head["commit_sha256"] = json!(hash(&encode(&next.commit)));
        }
        if stage >= 1 {
            next.allocations.insert(
                born_id(),
                wire::Allocation {
                    arena: "data".into(),
                    witness: witness.clone(),
                    bytes: if stage == 1 {
                        encode(&mark)
                    } else if stage >= 5 {
                        raw.clone()
                    } else {
                        vec![]
                    },
                },
            );
        }
        if stage >= 7 {
            for (aid, bytes) in &plan.files {
                next.allocations
                    .get_mut(aid)
                    .unwrap()
                    .bytes
                    .clone_from(bytes);
            }
        }
        if stage >= 4 {
            next.journal.push(frame.clone());
        }
        prior = next.clone();
        worlds.push(next);
    }
    // No candidate mutation is applied by the caller unless this complete dry run fits.
    let peak = phases::preflight(&old, &worlds, &o)?;
    // Marker file coexists at bound selection; charged as the same admitted newborn
    // allocation, and conservatively also counted in the standing control estimator.
    let mut marker_worlds = worlds.clone();
    for w in &mut marker_worlds[..3] {
        w.loose.insert(hash(&encode(&mark)), encode(&mark));
    }
    phases::preflight(&old, &marker_worlds, &o)?;
    Ok(Route {
        worlds,
        original: o,
        plan: generation,
        finalizer: f,
        prepared,
        old,
        peak,
    })
}

fn inspect(w: &wire::World) -> Result<Route> {
    let (o, p) = phases::selected_original(w)?;
    ensure(
        (o["original_codec"] == "photara.codec.ps2-single-birth-admission-v1"
            || o["original_codec"] == "photara.codec.ps2-single-birth-pending-admission-v1")
            && o["payload_recipe"]["codec"] == CODEC,
        "birth original codec",
    )?;
    if let Some(a) = w.allocations.get(&born_id()) {
        wire::fields(&a.witness, &["device", "inode"])?;
        ensure(
            number(&a.witness["inode"])? > 0
                && number(&a.witness["device"])? == number(&o["scope"]["directory"]["device"])?,
            "birth observed namespace/inode",
        )?;
    }
    let old = original_world(w, &o)?;
    let proof = wire::verify(&old)?;
    let intent = w.loose(&o["request"]["intent"])?;
    let receipt = if let Ok(r) = w.loose(&o["request"]["receipt"]) {
        r
    } else {
        w.loose(&p["finalization"])?["prepared_receipt"].clone()
    };
    let prepared = replay_prepare::regenerate(&old, &proof, &intent, &receipt)?;
    let binding = w.loose(&p["newborn_binding"])?;
    let witness = w.allocations.get(&born_id()).map_or_else(
        || json!({"device":"7","inode":"109"}),
        |a| a.witness.clone(),
    );
    let route = construct(old, &o, prepared, &witness)?;
    ensure(
        route.original == o && w.loose(&o["generation_plan"])? == route.plan,
        "birth exact immutable original",
    )?;
    let stage = STAGES
        .iter()
        .position(|s| p["stage"] == *s)
        .ok_or("birth stage")?;
    let expected = &route.worlds[stage];
    let (_, expected_phase) = phases::selected_original(expected)?;
    ensure(
        binding == expected.loose(&expected_phase["newborn_binding"])?
            && w.head == expected.head
            && w.commit == expected.commit
            && w.loose == expected.loose,
        "birth exact selected controls",
    )?;
    ensure(w.journal == expected.journal, "birth journal prerequisite")?;
    ensure(
        w.allocations.len() == expected.allocations.len()
            && w.allocations.iter().all(|(id, a)| {
                expected.allocations.get(id).is_some_and(|b| {
                    a.arena == b.arena && a.witness == b.witness && a.bytes == b.bytes
                })
            }),
        "birth exact authorized physical bytes",
    )?;
    if stage >= 8 && o["retention_intent"].is_null() {
        let overlay = w.loose(&w.commit["root_set"]["inventory"])?;
        let selection = wire::OperationSelection {
            expected_commit: expected.commit.clone(),
            controls: overlay["controls"].as_array().unwrap().clone(),
            packed_hold: o["old_hold"].clone(),
        };
        let proof =
            wire::verify_selected_operation(w, &route.old, &route.prepared.authored, &selection)?;
        ensure(
            proof.roles["active"].receipts.len() == 4,
            "birth actual original receipt closure",
        )?;
    }
    Ok(route)
}
fn seed() -> Result<(wire::World, Value, prepare::Prepared)> {
    let admitted = super::world("admitted");
    let (o, _) = phases::selected_original(&admitted)?;
    let old = phases::original_view(&admitted, &o)?;
    let proof = wire::verify(&old)?;
    let p = replay_prepare::regenerate(
        &old,
        &proof,
        &admitted.loose(&o["request"]["intent"])?,
        &admitted.loose(&o["request"]["receipt"])?,
    )?;
    Ok((old, o, p))
}
#[test]
fn one_born_sealed_selected_original_route() {
    let (old, o, prepared) = seed().unwrap();
    let route = construct(old, &o, prepared, &json!({"device":"7","inode":"109"})).unwrap();
    println!(
        "single-birth complete control preflight peak={} R={} F={} O={}",
        route.peak,
        route.original["reserve"],
        encode(&route.finalizer).len(),
        encode(&route.original).len()
    );
    for w in &route.worlds {
        inspect(w).unwrap();
    }
}

fn replace_refs(v: &mut Value, substitutions: &BTreeMap<String, Value>) {
    if v["kind"] == "json"
        && let Some(next) = v["sha256"].as_str().and_then(|s| substitutions.get(s))
    {
        *v = next.clone();
        return;
    }
    match v {
        Value::Object(m) => {
            for child in m.values_mut() {
                replace_refs(child, substitutions);
            }
        }
        Value::Array(a) => {
            for child in a {
                replace_refs(child, substitutions);
            }
        }
        _ => {}
    }
}
fn rebind(w: &wire::World, schema: &str, change: impl FnOnce(&mut Value)) -> wire::World {
    let mut originals = w
        .loose
        .iter()
        .map(|(k, v)| (k.clone(), wire::parse(v).unwrap()))
        .collect::<BTreeMap<_, _>>();
    let record = originals
        .values_mut()
        .find(|v| v["schema"]["id"] == schema)
        .unwrap();
    change(record);
    let mut mappings = BTreeMap::new();
    let mut values = originals.clone();
    for _ in 0..16 {
        values = originals.clone();
        for v in values.values_mut() {
            replace_refs(v, &mappings);
        }
        let next = values
            .iter()
            .map(|(k, v)| (k.clone(), reference(v)))
            .collect::<BTreeMap<_, _>>();
        if next == mappings {
            break;
        }
        mappings = next;
    }
    let mut changed = w.clone();
    changed.loose = values
        .values()
        .map(|v| (hash(&encode(v)), encode(v)))
        .collect();
    replace_refs(&mut changed.commit, &mappings);
    changed.head["commit_sha256"] = json!(hash(&encode(&changed.commit)));
    changed
}
#[test]
fn single_birth_full_worst_width_preflight_and_coherent_refusals() {
    let (old, o, p) = seed().unwrap();
    let worst = construct(
        old.clone(),
        &o,
        prepare::Prepared {
            authored: p.authored.clone(),
            intent: p.intent.clone(),
            receipt: p.receipt.clone(),
        },
        &json!({"device":u64::MAX.to_string(),"inode":u64::MAX.to_string()}),
    )
    .unwrap();
    let actual = construct(old, &o, p, &json!({"device":"7","inode":"109"})).unwrap();
    assert_eq!(worst.original, actual.original);
    assert_eq!(worst.peak, 131_072);
    println!(
        "worst-width single-birth F={} binding={} peak={}",
        encode(&worst.finalizer).len(),
        worst.worlds[8]
            .loose
            .values()
            .map(|b| wire::parse(b).unwrap())
            .find(|v| v["schema"]["id"] == "photara.storage.newborn-binding")
            .map(|v| encode(&v).len())
            .unwrap(),
        worst.peak
    );
    let complete = &actual.worlds[8];
    let changed = rebind(complete, "photara.storage.newborn-binding", |v| {
        v["slots"][0]["witness"]["inode"] = json!("110");
    });
    assert_eq!(
        inspect(&changed).err(),
        Some("birth exact selected controls")
    );
    let changed = rebind(complete, "photara.storage.finalization-control", |v| {
        v["sealed_allocations"][0]["charge"]["charged_high_water"] = json!("8192");
    });
    assert_eq!(
        inspect(&changed).err(),
        Some("birth exact selected controls")
    );
    let changed = rebind(complete, "photara.storage.original-admission", |v| {
        v["reserve"] = json!("1900543");
    });
    assert_eq!(
        inspect(&changed).err(),
        Some("birth exact immutable original")
    );
    for witness in [
        json!({"device":"8","inode":"109"}),
        json!({"device":"7","inode":"0"}),
    ] {
        let mut changed = actual.worlds[1].clone();
        changed.allocations.get_mut(&born_id()).unwrap().witness = witness;
        assert_eq!(
            inspect(&changed).err(),
            Some("birth observed namespace/inode")
        );
    }
    let mut partial = actual.worlds[1].clone();
    partial.allocations.get_mut(&born_id()).unwrap().bytes.pop();
    assert_eq!(
        inspect(&partial).err(),
        Some("birth exact authorized physical bytes")
    );
    let mut tail = actual.worlds[6].clone();
    tail.allocations.get_mut(&id(1002)).unwrap().bytes.push(0);
    assert_eq!(
        inspect(&tail).err(),
        Some("birth exact authorized physical bytes")
    );
    let mut extra = complete.clone();
    extra.loose.insert("f".repeat(64), b"{}".to_vec());
    assert_eq!(inspect(&extra).err(), Some("birth exact selected controls"));
}

/// Control sizing only: no pending semantic or evidence authority is granted here.
#[test]
fn embedded_pending_intent_size_preflight() {
    let (old, original, prepared) = seed().unwrap();
    let proof = wire::verify(&old).unwrap();
    let mut intent = obj(
        "unused",
        json!({
            "library_id":old.commit["root_set"]["library_id"],
            "bootstrap_sha256":old.commit["bootstrap_sha256"],
            "operation_id":prepared.receipt["operation_id"],
            "request_sha256":prepared.receipt["request_sha256"],
            "requirements":decoder::pending_requirements(&old,&proof).unwrap()
        }),
    );
    intent["schema"]["id"] = json!("photara.resource.pending-retention-intent");
    let route = construct(
        old,
        &original,
        prepared,
        &json!({"device":"7","inode":"109"}),
    )
    .unwrap();
    let before = encode(&route.original).len();
    let mut worlds = Vec::new();
    let mut previous = route.old.clone();
    for world in &route.worlds {
        let mut next = rebind(world, "photara.storage.original-admission", |o| {
            o["original_codec"] = json!("photara.codec.ps2-single-birth-pending-admission-v1");
            o["retention_intent"] = json!({"object":reference(&intent),"body":intent});
        });
        next.commit["parent"] = json!({"commit_id":previous.commit["commit_id"],"commit_sha256":hash(&encode(&previous.commit))});
        next.head["commit_sha256"] = json!(hash(&encode(&next.commit)));
        previous = next.clone();
        worlds.push(next);
    }
    let (changed, _) = phases::selected_original(&worlds[0]).unwrap();
    let after = encode(&changed).len();
    assert_eq!(rounded(before), rounded(after));
    let peak = phases::preflight(&route.old, &worlds, &changed).unwrap();
    let mark = marker(&changed, &route.plan);
    for world in &mut worlds[..3] {
        world.loose.insert(hash(&encode(&mark)), encode(&mark));
    }
    let marker_peak = phases::preflight(&route.old, &worlds, &changed).unwrap();
    assert_eq!(peak, 131_072);
    assert!(marker_peak <= 131_072);
    println!(
        "embedded intent={} O before={before} after={after} rounded={} adjacent-peak={peak} with-marker-peak={marker_peak}; no new loose role; pending semantic hook not implemented",
        encode(&intent).len(),
        rounded(after)
    );
}

pub(super) struct PendingInputs {
    pub(super) original: Value,
    pub(super) phase: Value,
    pub(super) old: wire::World,
    pub(super) prepared: prepare::Prepared,
    pub(super) selection: wire::OperationSelection,
}
pub(super) fn pending_inputs(w: &wire::World) -> Result<PendingInputs> {
    let route = inspect(w)?;
    let (original, phase) = phases::selected_original(w)?;
    ensure(
        !original["retention_intent"].is_null(),
        "pending original intent required",
    )?;
    let stage = STAGES
        .iter()
        .position(|s| phase["stage"] == *s)
        .ok_or("pending stage")?;
    let expected = &route.worlds[stage];
    let overlay = expected.loose(&expected.commit["root_set"]["inventory"])?;
    let selection = wire::OperationSelection {
        expected_commit: expected.commit.clone(),
        controls: overlay["controls"].as_array().unwrap().clone(),
        packed_hold: original["old_hold"].clone(),
    };
    Ok(PendingInputs {
        original,
        phase,
        old: route.old,
        prepared: route.prepared,
        selection,
    })
}
fn pending_route(mismatch_subset: bool) -> Result<Route> {
    let (old, mut original, prepared) = seed()?;
    let proof = wire::verify(&old)?;
    let requirements = if mismatch_subset {
        decoder::all_requirements(&old, &proof)?
    } else {
        decoder::pending_requirements(&old, &proof)?
    };
    let mut intent = obj(
        "unused",
        json!({"library_id":old.commit["root_set"]["library_id"],"bootstrap_sha256":old.commit["bootstrap_sha256"],"operation_id":prepared.receipt["operation_id"],"request_sha256":prepared.receipt["request_sha256"],"requirements":requirements}),
    );
    intent["schema"]["id"] = json!("photara.resource.pending-retention-intent");
    original["retention_intent"] = json!({"object":reference(&intent),"body":intent});
    construct(
        old,
        &original,
        prepared,
        &json!({"device":"7","inode":"109"}),
    )
}
pub(super) fn selected_pending(stage: &str, mismatch_subset: bool) -> Result<wire::World> {
    let route = pending_route(mismatch_subset)?;
    let index = STAGES
        .iter()
        .position(|s| *s == stage)
        .ok_or("pending selected stage")?;
    Ok(route.worlds[index].clone())
}

#[test]
fn exact_birth_pending_review_control_bytes() {
    let (old, o, p) = seed().unwrap();
    let birth = construct(old, &o, p, &json!({"device":"7","inode":"109"})).unwrap();
    let pending = pending_route(false).unwrap();
    let worst = construct(
        pending.old.clone(),
        &pending.original,
        prepare::Prepared {
            authored: pending.prepared.authored.clone(),
            intent: pending.prepared.intent.clone(),
            receipt: pending.prepared.receipt.clone(),
        },
        &json!({"device":u64::MAX.to_string(),"inode":u64::MAX.to_string()}),
    )
    .unwrap();
    assert_eq!(worst.original, pending.original);
    assert_eq!(worst.peak, 131_072);
    let mut records = BTreeMap::new();
    let mut selections = Vec::new();
    for (variant, route) in [("single-birth", birth), ("single-birth-pending", pending)] {
        for (stage, w) in STAGES.iter().zip(&route.worlds) {
            for bytes in w.loose.values().cloned().chain([
                encode(&w.head),
                encode(&w.commit),
                encode(&w.manifest),
            ]) {
                let value = wire::parse(&bytes).unwrap();
                assert_eq!(encode(&value), bytes);
                let digest = hash(&bytes);
                assert_eq!(reference(&value)["byte_length"], bytes.len().to_string());
                records.insert(digest,json!({"byte_length":bytes.len().to_string(),"canonical_utf8":String::from_utf8(bytes).unwrap()}));
            }
            selections.push(json!({"variant":variant,"stage":stage,"head":reference(&w.head),"commit":reference(&w.commit),"controls":w.loose.values().map(|b|reference(&wire::parse(b).unwrap())).collect::<Vec<_>>(),"allocations":w.allocations.iter().map(|(id,a)|json!({"allocation_id":id,"byte_length":a.bytes.len().to_string(),"sha256":hash(&a.bytes),"witness":a.witness})).collect::<Vec<_>>() }));
        }
    }
    let specimen = json!({"status":"unfrozen-selected-control-specimen","qualification":false,"physical_bytes":"Regenerate using exact original selected-phase inputs and decoder; only hashes included here.","records":records,"selections":selections});
    let bytes = serde_json::to_vec_pretty(&specimen).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/architecture/proposals/ps2/selected-phase/birth-pending-controls.json");
    if std::env::var_os("PHOTARA_REGENERATE_PHASE_CONTROLS").is_some() {
        std::fs::write(&path, &bytes).unwrap();
    }
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "exact canonical control corpus regeneration"
    );
    println!(
        "deduplicated selected control specimen bytes={} sha256={}",
        bytes.len(),
        hash(&bytes)
    );
}
