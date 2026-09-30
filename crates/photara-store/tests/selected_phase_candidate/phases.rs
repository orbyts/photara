//! Actual selected controls and original-owned suffix proof. No golden old-world fallback.
use super::{decoder, prepare, wire};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use wire::{Result, encode, ensure, fields, hash, key, number, reference, schema, text};
const STAGES: [&str; 7] = [
    "admitted",
    "journal-durable",
    "payload-durable",
    "finalizer-selected",
    "finalizer-durable",
    "published",
    "clean",
];
const GROWTH: u64 = 1_835_008;
const RESERVE: u64 = 1_900_544;
const CLEANUP: u64 = 65_536;
fn obj(name: &str, mut v: Value) -> Value {
    v["schema"] = json!({"id":name,"version":1});
    v["project_id"] = json!(wire::PROJECT);
    v["extensions"] = json!({});
    v
}
fn uid(n: u64) -> String {
    format!("96000000-0000-4000-8000-{n:012}")
}
fn arr(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("phase array")
}
fn frame(receipt: &Value) -> Value {
    json!({"schema":{"id":"photara.package.accepted-journal-frame","version":1},"journal_id":receipt["journal_id"],"sequence":receipt["journal_sequence"],"operation_id":receipt["operation_id"],"request_sha256":receipt["request_sha256"],"receipt_sha256":reference(receipt)["sha256"]})
}
fn end(v: &Value) -> Result<usize> {
    usize::try_from(number(v)?).map_err(|_| "phase offset bound")
}
fn span(a: &wire::Allocation, id: &str, lo: usize, hi: usize) -> Result<Value> {
    ensure(lo <= hi && hi <= a.bytes.len(), "phase span bounds")?;
    let mut p = lo;
    let mut count = 0u64;
    while p < hi {
        ensure(
            hi - p >= 16 && &a.bytes[p..p + 8] == b"PS2PKD01",
            "phase complete frame",
        )?;
        p = p
            .checked_add(16)
            .and_then(|x| {
                x.checked_add(
                    u32::from_le_bytes(a.bytes[p + 12..p + 16].try_into().unwrap()) as usize,
                )
            })
            .ok_or("phase frame overflow")?;
        count += 1;
    }
    ensure(p == hi, "phase whole-frame split")?;
    Ok(
        json!({"allocation_id":id,"arena":a.arena,"witness":a.witness,"original_end":lo.to_string(),"final_end":hi.to_string(),"frame_count":count.to_string(),"framed_sha256":hash(&a.bytes[lo..hi])}),
    )
}
fn split(old: &wire::World, plan: &decoder::Plan) -> Result<(Vec<Value>, Vec<Value>)> {
    let mut payload = Vec::new();
    let mut writes = Vec::new();
    for (id, b) in &plan.files {
        let prior = &old.allocations[id];
        if b == &prior.bytes {
            continue;
        }
        let mut a = prior.clone();
        a.bytes.clone_from(b);
        let start = prior.bytes.len();
        let mut stop = start;
        if a.arena == "data" {
            while stop < b.len() {
                let tag = b[stop + 8];
                let next = stop
                    + 16
                    + u32::from_le_bytes(b[stop + 12..stop + 16].try_into().unwrap()) as usize;
                ensure(next <= b.len(), "split frame bound")?;
                if tag == 0
                    || wire::parse(&b[stop + 16..next])?["schema"]["id"]
                        == "photara.storage.allocation-claim"
                {
                    break;
                }
                stop = next;
            }
        }
        payload.push(span(&a, id, start, stop)?);
        writes.push(span(&a, id, stop, b.len())?);
    }
    Ok((payload, writes))
}
/// Historical logical view from embedded exact controls plus still-present actual original prefixes.
/// This is NOT current physical accounting: every remaining live suffix is checked separately.
pub(super) fn original_view(w: &wire::World, o: &Value) -> Result<wire::World> {
    let mut old = w.clone();
    old.head = o["old"]["head"].clone();
    old.commit = o["old"]["commit"].clone();
    old.loose.clear();
    for name in ["ledger", "envelope", "overlay"] {
        let v = &o["old"][name];
        old.loose.insert(hash(&encode(v)), encode(v));
    }
    let mut seen = BTreeSet::new();
    for r in arr(&o["payload_recipe"]["allocations"])? {
        let id = text(&r["allocation_id"])?;
        ensure(seen.insert(id), "unique original allocation recipe")?;
        let a = old
            .allocations
            .get_mut(id)
            .ok_or("original allocation absent")?;
        let n = end(&r["original_end"])?;
        ensure(
            a.witness == r["witness"]
                && a.arena == r["arena"]
                && a.bytes.len() >= n
                && hash(&a.bytes[..n]) == r["original_sha256"],
            "actual original prefix/witness",
        )?;
        a.bytes.truncate(n);
    }
    let p = wire::verify(&old)?;
    old.journal = p.roles["active"].receipts.iter().map(frame).collect();
    Ok(old)
}
#[allow(
    clippy::many_single_char_names,
    reason = "O/P and control envelope symbols mirror the selected protocol"
)]
pub(super) fn selected_original(w: &wire::World) -> Result<(Value, Value)> {
    let root = &w.commit["root_set"];
    let e = w.loose(&root["placement"]["accounting"])?;
    let h = w.loose(&e["holds"])?;
    schema(
        &h,
        "photara.storage.hold-leaf",
        1,
        &["count", "reserved", "consumed", "remaining", "entries"],
    )?;
    ensure(
        h["count"] == "1" && arr(&h["entries"])?.len() == 1,
        "phase sole original hold",
    )?;
    let entry = &h["entries"][0];
    fields(entry, &["token", "original", "phase"])?;
    let o = w.loose(&entry["original"])?;
    let p = w.loose(&entry["phase"])?;
    ensure(
        entry["token"] == o["token"] && p["original"] == reference(&o),
        "phase original identity",
    )?;
    Ok((o, p))
}
pub(super) fn packed_value(plan: &decoder::Plan, r: &Value) -> Result<Value> {
    for bytes in plan.files.values() {
        let mut p = 0;
        while p < bytes.len() {
            ensure(
                bytes.len() - p >= 16
                    && &bytes[p..p + 8] == b"PS2PKD01"
                    && bytes[p + 8] <= 3
                    && bytes[p + 9..p + 12] == [0, 0, 0],
                "phase supplied frame header",
            )?;
            let e = p
                .checked_add(16)
                .and_then(|v| {
                    v.checked_add(
                        u32::from_le_bytes(bytes[p + 12..p + 16].try_into().unwrap()) as usize,
                    )
                })
                .ok_or("phase supplied frame overflow")?;
            ensure(e <= bytes.len(), "phase supplied frame end")?;
            if bytes[p + 8] != 0 {
                let raw = &bytes[p + 16..e];
                if hash(raw) == r["sha256"] {
                    let v = wire::parse(raw)?;
                    ensure(reference(&v) == *r, "phase exact packed ref")?;
                    return Ok(v);
                }
            }
            p = e;
        }
    }
    Err("phase packed record absent")
}
pub(super) fn publish_control(
    old: &wire::World,
    prior: &wire::World,
    root: Value,
    controls: &[Value],
    overlay: &Value,
    id: u64,
    active: Option<&Value>,
) -> Result<wire::World> {
    let mut w = old.clone();
    w.commit["root_set"] = root;
    if let Some(s) = active {
        for name in ["authored", "history", "inventory"] {
            w.commit[name] = s[name].clone();
        }
    }
    w.commit["commit_id"] = json!(uid(id));
    w.commit["write_id"] = json!(uid(id + 1));
    w.commit["package_revision"] =
        json!((number(&prior.commit["package_revision"])? + 1).to_string());
    w.commit["parent"] =
        json!({"commit_id":prior.commit["commit_id"],"commit_sha256":hash(&encode(&prior.commit))});
    w.head["commit_id"] = w.commit["commit_id"].clone();
    w.head["commit_sha256"] = json!(hash(&encode(&w.commit)));
    w.loose = controls
        .iter()
        .chain([overlay])
        .map(|v| (hash(&encode(v)), encode(v)))
        .collect();
    Ok(w)
}
struct Derived {
    worlds: Vec<wire::World>,
    finalizer: Value,
    prepared: prepare::Prepared,
    old: wire::World,
    peak: u64,
}
fn ledger_after(old: &wire::World, o: &Value, plan: &decoder::Plan) -> Result<Value> {
    let mut ledger = o["old"]["ledger"].clone();
    let mut growth = 0u64;
    for tip in ledger["tips"].as_array_mut().ok_or("phase tips")? {
        let id = text(&tip["allocation_id"])?;
        ensure(old.allocations.contains_key(id), "phase original tip")?;
        let n = plan.files[id].len() as u64;
        let charge = n.checked_add(4095).ok_or("phase rounding")? / 4096 * 4096;
        growth = growth
            .checked_add(
                charge
                    .checked_sub(number(&tip["registered_charge"])?)
                    .ok_or("phase no charge decrease")?,
            )
            .ok_or("phase charge overflow")?;
        tip["extent"] = json!(n.to_string());
        tip["registered_charge"] = json!(charge.to_string());
        tip["observation"]["measured_extent"] = json!(n.to_string());
        tip["observation"]["charged_high_water"] = json!(charge.to_string());
    }
    ensure(
        growth == GROWTH
            && number(&o["reserve"])? == RESERVE
            && number(&o["cleanup_bound"])? == CLEANUP,
        "phase original finite budget",
    )?;
    ledger["total_charge"] = json!((number(&ledger["total_charge"])? + growth).to_string());
    Ok(ledger)
}
#[allow(
    clippy::too_many_lines,
    reason = "Keep independent original regeneration and finite admission preflight in order"
)]
fn derive(
    w: &wire::World,
    o: &Value,
    prepared: prepare::Prepared,
    old: wire::World,
) -> Result<Derived> {
    let oldproof = wire::verify(&old)?;
    let plan = decoder::regenerate(&old, &oldproof, &prepared)?;
    ensure(
        plan.target == o["semantic_target"]
            && plan.base_inventory == o["payload_recipe"]["base_inventory"]
            && plan.root_placements == o["payload_recipe"]["target_placement"]["root_placements"],
        "original decoder exact target",
    )?;
    ensure(
        o["original_codec"] == "photara.codec.ps2-same-tip-admission-v1"
            && o["payload_recipe"]["codec"] == "photara.codec.ps2-same-tip-layout-v1",
        "original replay codec supported",
    )?;
    for r in arr(&o["payload_recipe"]["allocations"])? {
        let id = text(&r["allocation_id"])?;
        let mut a = old.allocations[id].clone();
        a.bytes.clone_from(&plan.files[id]);
        let mut exact = span(&a, id, end(&r["original_end"])?, a.bytes.len())?;
        exact["original_sha256"] = json!(hash(&old.allocations[id].bytes));
        ensure(exact == *r, "original exact full append recipe")?;
    }
    let (payload, writes) = split(&old, &plan)?;
    let target = &o["semantic_target"];
    let mut projection = target.clone();
    projection["retention_evidence"] = old.commit["root_set"]["retention_evidence"].clone();
    projection["base_inventory"] = plan.base_inventory.clone();
    projection["placement"] = o["payload_recipe"]["target_placement"].clone();
    projection["active_state"] = packed_value(&plan, &target["active"])?;
    let f = obj(
        "photara.storage.finalization-control",
        json!({"token":o["token"],"original":reference(o),"prepared_receipt":prepared.receipt,"generation_plan":null,"newborn_binding":null,"payload_completion":payload,"sealed_allocations":[],"recipe_codec":o["payload_recipe"]["codec"],"writes":writes,"target_projection":projection}),
    );
    ensure(
        encode(&f).len() as u64 <= number(&o["control_bounds"]["finalization_bytes"])?,
        "original finalizer byte bound",
    )?;
    let ledger = ledger_after(&old, o, &plan)?;
    let state = packed_value(&plan, &target["active"])?;
    let mut worlds = control_chain(
        &old,
        o,
        &prepared,
        &f,
        &ledger,
        &state,
        number(&packed_value(&plan, &plan.base_inventory)?["count"])?,
    )?;
    for (i, next) in worlds.iter_mut().enumerate() {
        if i >= 2 {
            for r in &payload {
                let id = text(&r["allocation_id"])?;
                next.allocations.get_mut(id).unwrap().bytes =
                    plan.files[id][..end(&r["final_end"])?].to_vec();
            }
        }
        if i >= 4 {
            for (id, b) in &plan.files {
                next.allocations.get_mut(id).unwrap().bytes.clone_from(b);
            }
        }
    }
    // Revalidate the actual original admission predicate on the reconstructed original view.
    // This is admission evidence, not a claim that mutated current bytes equal old bytes.
    let mut future = worlds[5].clone();
    let envelope = obj(
        "photara.storage.accounting-envelope",
        json!({"ledger":reference(&ledger),"holds":o["old_hold"]}),
    );
    let mut refs = vec![reference(&ledger), reference(&envelope)];
    refs.sort_by_key(|r| key(r).unwrap());
    let overlay = obj(
        "photara.package.inventory-overlay",
        json!({"base":plan.base_inventory,"controls":refs,"count":(number(&packed_value(&plan,&plan.base_inventory)?["count"])?+2).to_string()}),
    );
    let mut root = future.commit["root_set"].clone();
    root["inventory"] = reference(&overlay);
    root["placement"]["accounting"] = reference(&envelope);
    let skeleton = publish_control(
        &old,
        &old,
        root,
        &[ledger, envelope],
        &overlay,
        9001,
        Some(&state),
    )?;
    future.head = skeleton.head;
    future.commit = skeleton.commit;
    future.loose = skeleton.loose;
    let fp = wire::verify_transition(&future, &old, &prepared.authored)?;
    super::admission::verify(
        &json!({"dry_run":{"recipe":o["payload_recipe"]["allocations"]}}),
        &old,
        &worlds[0],
        &future,
        &oldproof,
        &prepared,
        &fp,
    )?;
    let peak = preflight(&old, &worlds, o)?;
    // The supplied bytes, not this regenerated candidate, remain the verification subject.
    ensure(
        w.manifest == old.manifest,
        "phase unchanged original manifest",
    )?;
    Ok(Derived {
        worlds,
        finalizer: f,
        prepared,
        old,
        peak,
    })
}
fn rounded(n: usize) -> Result<u64> {
    (n as u64)
        .checked_add(4095)
        .map(|n| n / 4096 * 4096)
        .ok_or("phase control rounding")
}
pub(super) fn preflight(old: &wire::World, worlds: &[wire::World], o: &Value) -> Result<u64> {
    let mut peak = 0;
    let mut prior = old;
    for next in worlds {
        let mut files = BTreeMap::new();
        for w in [prior, next] {
            for (k, b) in &w.loose {
                files.insert(k.clone(), b.len());
            }
            for v in [&w.manifest, &w.head, &w.commit] {
                files.insert(hash(&encode(v)), encode(v).len());
            }
        }
        let intent = obj(
            "photara.storage.admission-selector",
            json!({"token":o["token"],"original":reference(o),"old_head_sha256":hash(&encode(&prior.head)),"next_head_sha256":hash(&encode(&next.head))}),
        );
        ensure(
            encode(&intent).len() <= 4096 && encode(&next.head).len() <= 4096,
            "phase exact selector control bound",
        )?;
        let charge = files.values().try_fold(12_288u64, |a, n| {
            a.checked_add(rounded(*n)?)
                .ok_or("phase coexistence overflow")
        })?;
        ensure(
            charge <= number(&o["control_bounds"]["aggregate_control_bytes"])?
                && files.len() as u64 + 3
                    <= number(&o["control_bounds"]["aggregate_control_count"])?,
            "original simultaneous control bound",
        )?;
        peak = peak.max(charge);
        prior = next;
    }
    Ok(peak)
}
pub(super) struct Proof {
    pub stage: usize,
    pub consumed: u64,
    pub remaining: u64,
    pub control_peak: u64,
    pub receipts: Vec<Value>,
}
fn inspect(w: &wire::World) -> Result<(Derived, usize)> {
    let (o, p) = selected_original(w)?;
    original_shape(&o)?;
    let stage = STAGES
        .iter()
        .position(|s| p["stage"] == *s)
        .ok_or("supported selected phase")?;
    // Codec refusal occurs before a reconstruction attempt or any proposed next effect.
    ensure(
        o["original_codec"] == "photara.codec.ps2-same-tip-admission-v1"
            && o["payload_recipe"]["codec"] == "photara.codec.ps2-same-tip-layout-v1",
        "original replay codec supported",
    )?;
    let old = original_view(w, &o)?;
    let oldproof = wire::verify(&old)?;
    let intent = w.loose(&o["request"]["intent"])?;
    let receipt = if let Ok(v) = w.loose(&o["request"]["receipt"]) {
        v
    } else {
        // Receipt is packed after publication; resolve from actual complete current frames.
        let plan = decoder::Plan {
            files: w
                .allocations
                .iter()
                .map(|(id, a)| (id.clone(), a.bytes.clone()))
                .collect(),
            target: Value::Null,
            base_inventory: Value::Null,
            root_placements: Value::Null,
        };
        packed_value(&plan, &o["request"]["receipt"])?
    };
    let prepared = super::replay_prepare::regenerate(&old, &oldproof, &intent, &receipt)?;
    let d = derive(w, &o, prepared, old)?;
    let expected = &d.worlds[stage];
    ensure(
        w.head == expected.head && w.commit == expected.commit && w.loose == expected.loose,
        "exact original-derived selected phase controls",
    )?;
    ensure(
        w.source == expected.source
            && w.source_witnesses == expected.source_witnesses
            && (w.journal == expected.journal || (stage == 0 && w.journal == d.worlds[1].journal)),
        "phase exact source/journal preservation",
    )?;
    ensure(
        w.allocations.keys().eq(expected.allocations.keys()),
        "phase exact allocation set",
    )?;
    let maximum = match stage {
        1 => &d.worlds[2],
        3 => &d.worlds[4],
        _ => expected,
    };
    for (id, a) in &w.allocations {
        let min = &expected.allocations[id];
        let max = &maximum.allocations[id];
        ensure(
            a.arena == min.arena && a.witness == min.witness,
            "phase original allocation witness",
        )?;
        ensure(
            a.bytes.len() >= min.bytes.len()
                && a.bytes.len() <= max.bytes.len()
                && a.bytes == max.bytes[..a.bytes.len()],
            "original-authorized exact live suffix",
        )?;
    }
    if stage >= 5 {
        let overlay = w.loose(&w.commit["root_set"]["inventory"])?;
        let selection = wire::OperationSelection {
            expected_commit: expected.commit.clone(),
            controls: arr(&overlay["controls"])?.clone(),
            packed_hold: d
                .old
                .loose(&d.old.commit["root_set"]["placement"]["accounting"])?["holds"]
                .clone(),
        };
        let p = wire::verify_selected_operation(w, &d.old, &d.prepared.authored, &selection)?;
        ensure(
            p.total
                == number(
                    &d.old.loose(
                        &d.old
                            .loose(&d.old.commit["root_set"]["placement"]["accounting"])?["ledger"],
                    )?["total_charge"],
                )? + GROWTH,
            "actual published once-only charge",
        )?;
        ensure(
            p.roles["active"].receipts.len() == 4
                && p.roles["active"].receipts.last() == Some(&d.prepared.receipt),
            "actual selected fourth receipt",
        )?;
    }
    Ok((d, stage))
}
pub(super) fn verify(w: &wire::World) -> Result<Proof> {
    let (d, stage) = inspect(w)?;
    let mut receipts = wire::verify(&d.old)?.roles["active"].receipts.clone();
    if stage >= 5 {
        receipts.push(d.prepared.receipt);
    }
    Ok(Proof {
        stage,
        consumed: if stage >= 5 { GROWTH } else { 0 },
        remaining: if stage == 6 {
            0
        } else if stage == 5 {
            CLEANUP
        } else {
            RESERVE
        },
        control_peak: d.peak,
        receipts,
    })
}
/// Returns the next complete modeled effect/selector state only after validating all current bytes.
/// The function performs no I/O; callers compare/process-cut the returned finite append recipe.
pub(super) fn advance(w: &wire::World) -> Result<wire::World> {
    let (d, stage) = inspect(w)?;
    Ok(d.worlds[(stage + 1).min(6)].clone())
}
pub(super) fn retry(w: &wire::World, original: &Value, request: &str) -> Result<(Value, u64, u64)> {
    let (d, stage) = inspect(w)?;
    ensure(stage == 6, "terminal clean retry required")?;
    let (o, _) = selected_original(w)?;
    ensure(
        reference(&o) == *original && d.prepared.receipt["request_sha256"] == request,
        "exact original terminal retry",
    )?;
    Ok((d.prepared.receipt, 0, 0))
}

fn control_chain(
    old: &wire::World,
    o: &Value,
    prepared: &prepare::Prepared,
    f: &Value,
    ledger: &Value,
    state: &Value,
    basecount: u64,
) -> Result<Vec<wire::World>> {
    let mut worlds = Vec::new();
    let mut prior = old.clone();
    for (i, stage) in STAGES.iter().enumerate() {
        let published = i >= 5;
        let consumed = if published { GROWTH } else { 0 };
        let remaining = if i == 6 { 0 } else { RESERVE - consumed };
        let mut phase = obj(
            "photara.storage.operation-phase",
            json!({"token":o["token"],"original":reference(o),"stage":stage,"consumed":consumed.to_string(),"remaining":remaining.to_string(),"newborn_binding":null,"finalization":if i>=3{reference(f)}else{Value::Null},"cleanup":if i==6{json!({"remaining_roles":[],"directory_barrier":true,"released":CLEANUP.to_string()})}else{Value::Null}}),
        );
        if i >= 1 {
            phase["journal_completion"] = json!({"frame":frame(&prepared.receipt),"barrier":true});
        }
        ensure(
            encode(&phase).len() as u64 <= number(&o["control_bounds"]["phase_bytes"])?,
            "original phase byte bound",
        )?;
        let holds = obj(
            "photara.storage.hold-leaf",
            json!({"count":"1","reserved":RESERVE.to_string(),"consumed":consumed.to_string(),"remaining":remaining.to_string(),"entries":[{"token":o["token"],"original":reference(o),"phase":reference(&phase)}]}),
        );
        let l = if published {
            ledger.clone()
        } else {
            o["old"]["ledger"].clone()
        };
        let envelope = obj(
            "photara.storage.accounting-envelope",
            json!({"ledger":reference(&l),"holds":reference(&holds)}),
        );
        let mut controls = vec![
            l,
            envelope.clone(),
            o.clone(),
            phase,
            holds,
            prepared.intent.clone(),
        ];
        if !published {
            controls.push(prepared.receipt.clone());
        }
        if i >= 3 {
            controls.push(f.clone());
        }
        let base = if published {
            o["payload_recipe"]["base_inventory"].clone()
        } else {
            o["old"]["overlay"]["base"].clone()
        };
        let count = if published {
            basecount
        } else {
            number(&o["old"]["overlay"]["count"])?
                - arr(&o["old"]["overlay"]["controls"])?.len() as u64
        };
        let mut refs = controls.iter().map(reference).collect::<Vec<_>>();
        refs.sort_by_key(|r| key(r).unwrap());
        let overlay = obj(
            "photara.package.inventory-overlay",
            json!({"base":base,"controls":refs,"count":(count+controls.len() as u64).to_string()}),
        );
        let mut root = old.commit["root_set"].clone();
        root["inventory"] = reference(&overlay);
        root["placement"]["accounting"] = reference(&envelope);
        if published {
            for (k, v) in o["semantic_target"].as_object().ok_or("phase target")? {
                root[k] = v.clone();
            }
            root["placement"]["generation"] = json!("2");
            root["placement"]["root_placements"] =
                o["payload_recipe"]["target_placement"]["root_placements"].clone();
        }
        let mut next = publish_control(
            old,
            &prior,
            root,
            &controls,
            &overlay,
            if i == 0 { 9101 } else { 9200 + i as u64 * 2 },
            published.then_some(state),
        )?;
        if i >= 1 {
            next.journal.push(frame(&prepared.receipt));
        }
        prior = next.clone();
        worlds.push(next);
    }
    Ok(worlds)
}
/// Own recovery readability and selected control commitments only. Missing foreign
/// payloads remain unverified; no replay, admission, global charge or cleanup authority.
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    reason = "Keep bounded O/P/F metadata-only recovery obligations explicit in one pass"
)]
pub(super) fn recovery(w: &wire::World) -> Result<wire::RoleProof> {
    let (o, p) = selected_original(w)?;
    original_shape(&o)?;
    let stage = STAGES
        .iter()
        .position(|s| p["stage"] == *s)
        .ok_or("supported selected phase")?;
    ensure(stage >= 5, "published recovery only")?;
    schema(
        &o,
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
        o["original_codec"] == "photara.codec.ps2-same-tip-admission-v1"
            && o["payload_recipe"]["codec"] == "photara.codec.ps2-same-tip-layout-v1"
            && o["generation_plan"].is_null()
            && o["retention_intent"].is_null(),
        "readonly original codec/null dispatch",
    )?;
    let f = w.loose(&p["finalization"])?;
    schema(
        &f,
        "photara.storage.finalization-control",
        1,
        &[
            "token",
            "original",
            "prepared_receipt",
            "generation_plan",
            "newborn_binding",
            "payload_completion",
            "sealed_allocations",
            "recipe_codec",
            "writes",
            "target_projection",
        ],
    )?;
    ensure(
        encode(&f).len() as u64 <= number(&o["control_bounds"]["finalization_bytes"])?,
        "original finalizer byte bound",
    )?;
    let receipt = &f["prepared_receipt"];
    let active = &f["target_projection"]["active_state"];
    state_header(
        active,
        &o["old"]["commit"]["bootstrap_sha256"],
        &o["old"]["commit"]["root_set"]["library_id"],
    )?;
    wire::receipt(receipt, text(&o["old"]["commit"]["bootstrap_sha256"])?)?;
    let previous = arr(&o["old"]["states"])?
        .iter()
        .find(|r| r["root"] == o["old"]["commit"]["root_set"]["active"])
        .ok_or("readonly old active state")?;
    let before = &previous["value"];
    ensure(
        receipt["before"]
            == json!({"revision":before["authored_revision"],"digest":before["authored"]["sha256"]})
            && receipt["after"]
                == json!({"revision":active["authored_revision"],"digest":active["authored"]["sha256"]})
            && number(&before["accepted"]["through_ordinal"])?.checked_add(1)
                == Some(number(&receipt["acceptance_ordinal"])?)
            && receipt["journal_id"] == before["journal_inclusion"]["journal_id"]
            && number(&receipt["journal_sequence"])?
                > number(&before["journal_inclusion"]["through_sequence"])?
            && active["predecessor"]
                == json!({"root_sha256":previous["root"]["sha256"],"authored_revision":before["authored_revision"]}),
        "readonly original receipt/state coordinate",
    )?;
    ensure(
        f["token"] == o["token"]
            && f["original"] == reference(&o)
            && f["generation_plan"].is_null()
            && f["newborn_binding"].is_null()
            && f["sealed_allocations"] == json!([])
            && f["recipe_codec"] == o["payload_recipe"]["codec"]
            && reference(receipt) == o["request"]["receipt"]
            && reference(active) == o["semantic_target"]["active"],
        "readonly original finalization commitments",
    )?;
    let mut target = o["semantic_target"].clone();
    target["retention_evidence"] = o["old"]["commit"]["root_set"]["retention_evidence"].clone();
    target["base_inventory"] = o["payload_recipe"]["base_inventory"].clone();
    target["placement"] = o["payload_recipe"]["target_placement"].clone();
    target["active_state"] = active.clone();
    ensure(
        f["target_projection"] == target,
        "readonly exact target projection",
    )?;
    let intent = w.loose(&o["request"]["intent"])?;
    ensure(
        hash(&encode(&intent)) == o["request"]["request_sha256"]
            && receipt["operation_id"] == o["request"]["operation_id"]
            && receipt["request_sha256"] == o["request"]["request_sha256"],
        "readonly original request identity",
    )?;
    let mut old = w.clone();
    old.head = o["old"]["head"].clone();
    old.commit = o["old"]["commit"].clone();
    old.loose.clear();
    for name in ["ledger", "envelope", "overlay"] {
        let v = &o["old"][name];
        old.loose.insert(hash(&encode(v)), encode(v));
    }
    ensure(
        old.head["commit_sha256"] == hash(&encode(&old.commit))
            && old.commit["bootstrap_sha256"] == hash(&encode(&w.manifest))
            && reference(&o["old"]["envelope"])
                == old.commit["root_set"]["placement"]["accounting"]
            && reference(&o["old"]["ledger"]) == o["old"]["envelope"]["ledger"]
            && reference(&o["old"]["overlay"]) == old.commit["root_set"]["inventory"]
            && o["old_hold"] == o["old"]["envelope"]["holds"],
        "readonly original control commitments",
    )?;
    ensure(
        o["semantic_target"]["recovery"] == old.commit["root_set"]["active"]
            && o["semantic_target"]["pinned_roots"] == old.commit["root_set"]["pinned_roots"]
            && o["semantic_target"]["conversion_source"]
                == old.commit["root_set"]["conversion_source"],
        "readonly retained original roots",
    )?;
    let mut ledger = o["old"]["ledger"].clone();
    let mut growth = 0u64;
    let recipes = arr(&o["payload_recipe"]["allocations"])?;
    let payload = arr(&f["payload_completion"])?;
    let writes = arr(&f["writes"])?;
    ensure(
        recipes.len() == 7 && payload.len() == 7 && writes.len() == 7,
        "readonly exact original manifest set",
    )?;
    let mut recipe_ids = BTreeSet::new();
    let mut previous_id = None;
    for ((r, a), b) in recipes.iter().zip(payload).zip(writes) {
        let id = text(&r["allocation_id"])?;
        ensure(
            previous_id.is_none_or(|p| p < id) && recipe_ids.insert(id.to_owned()),
            "readonly sorted unique original manifests",
        )?;
        previous_id = Some(id);
        fields(
            r,
            &[
                "allocation_id",
                "arena",
                "witness",
                "original_end",
                "original_sha256",
                "final_end",
                "frame_count",
                "framed_sha256",
            ],
        )?;
        for s in [a, b] {
            fields(
                s,
                &[
                    "allocation_id",
                    "arena",
                    "witness",
                    "original_end",
                    "final_end",
                    "frame_count",
                    "framed_sha256",
                ],
            )?;
            ensure(
                s["allocation_id"] == r["allocation_id"]
                    && s["arena"] == r["arena"]
                    && s["witness"] == r["witness"],
                "readonly original split identity",
            )?;
        }
        ensure(
            a["original_end"] == r["original_end"]
                && a["final_end"] == b["original_end"]
                && b["final_end"] == r["final_end"]
                && number(&a["frame_count"])?.checked_add(number(&b["frame_count"])?)
                    == Some(number(&r["frame_count"])?),
            "readonly contiguous original split",
        )?;
        let id = text(&r["allocation_id"])?;
        if let Some(actual) = w.allocations.get(id) {
            let start = end(&r["original_end"])?;
            let finish = end(&r["final_end"])?;
            ensure(
                actual.bytes.len() == finish
                    && start <= finish
                    && actual.witness == r["witness"]
                    && actual.arena == r["arena"]
                    && hash(&actual.bytes[..start]) == r["original_sha256"]
                    && hash(&actual.bytes[start..]) == r["framed_sha256"],
                "readonly supplied original suffix",
            )?;
            ensure(
                span(actual, id, start, end(&a["final_end"])?)? == *a
                    && span(actual, id, end(&b["original_end"])?, finish)? == *b,
                "readonly supplied split bytes",
            )?;
        }
        let tip = ledger["tips"]
            .as_array_mut()
            .ok_or("readonly tips")?
            .iter_mut()
            .find(|t| t["allocation_id"] == r["allocation_id"])
            .ok_or("readonly exact original tip")?;
        let n = number(&r["final_end"])?;
        let charge = n.checked_add(4095).ok_or("readonly rounding")? / 4096 * 4096;
        growth = growth
            .checked_add(
                charge
                    .checked_sub(number(&tip["registered_charge"])?)
                    .ok_or("readonly highwater decrease")?,
            )
            .ok_or("readonly growth overflow")?;
        tip["extent"] = json!(n.to_string());
        tip["registered_charge"] = json!(charge.to_string());
        tip["observation"]["measured_extent"] = json!(n.to_string());
        tip["observation"]["charged_high_water"] = json!(charge.to_string());
    }
    ensure(
        recipe_ids
            == arr(&o["old"]["ledger"]["tips"])?
                .iter()
                .map(|t| text(&t["allocation_id"]).map(str::to_owned))
                .collect::<Result<BTreeSet<_>>>()?,
        "readonly exact original manifest tip set",
    )?;
    ensure(
        growth == GROWTH
            && number(&o["reserve"])? == RESERVE
            && number(&o["cleanup_bound"])? == CLEANUP,
        "readonly original budget",
    )?;
    ledger["total_charge"] = json!((number(&ledger["total_charge"])? + growth).to_string());
    let supplied = decoder::Plan {
        files: w
            .allocations
            .iter()
            .map(|(id, a)| (id.clone(), a.bytes.clone()))
            .collect(),
        target: Value::Null,
        base_inventory: Value::Null,
        root_placements: Value::Null,
    };
    let pins = packed_value(&supplied, &o["old"]["commit"]["root_set"]["pinned_roots"])?;
    schema(
        &pins,
        "photara.package.retained-root-leaf",
        1,
        &["count", "entries"],
    )?;
    let mut exact_roots = BTreeSet::from([
        key(&o["old"]["commit"]["root_set"]["active"])?,
        key(&o["old"]["commit"]["root_set"]["recovery"])?,
    ]);
    for pin in arr(&pins["entries"])? {
        fields(pin, &["pin_id", "reason", "root"])?;
        uuid(&pin["pin_id"])?;
        exact_roots.insert(key(&pin["root"])?);
    }
    ensure(
        number(&pins["count"])? == arr(&pins["entries"])?.len() as u64
            && exact_roots
                == arr(&o["old"]["states"])?
                    .iter()
                    .map(|row| key(&row["root"]))
                    .collect::<Result<BTreeSet<_>>>()?,
        "readonly exact original pinned state set",
    )?;
    let count = number(&packed_value(&supplied, &o["payload_recipe"]["base_inventory"])?["count"])?;
    let prepared = prepare::Prepared {
        authored: BTreeMap::new(),
        intent,
        receipt: receipt.clone(),
    };
    let chain = control_chain(&old, &o, &prepared, &f, &ledger, active, count)?;
    preflight(&old, &chain, &o)?;
    let expected = &chain[stage];
    ensure(
        w.head == expected.head && w.commit == expected.commit && w.loose == expected.loose,
        "readonly exact selected phase controls",
    )?;
    let overlay = w.loose(&w.commit["root_set"]["inventory"])?;
    let selection = wire::OperationSelection {
        expected_commit: expected.commit.clone(),
        controls: arr(&overlay["controls"])?.clone(),
        packed_hold: o["old_hold"].clone(),
    };
    let proof = wire::recovery_operation(w, &selection)?;
    let original_state = arr(&o["old"]["states"])?
        .iter()
        .find(|r| r["root"] == o["semantic_target"]["recovery"])
        .ok_or("readonly original recovery body")?;
    ensure(
        reference(&proof.state) == o["semantic_target"]["recovery"]
            && proof.state == original_state["value"]
            && proof.receipts.len() == 3,
        "readonly exact original recovery",
    )?;
    Ok(proof)
}
/// Reconcile only exact modeled selector sidecars. Unknown bytes or roles fence without effects.
/// All immutable candidate objects coexist under preflight; this returns the selected result,
/// while dropping these three known transient roles models barriered selector cleanup.
pub(super) fn reconcile(
    w: &wire::World,
    sidecars: &BTreeMap<String, Vec<u8>>,
) -> Result<wire::World> {
    let (d, current) = inspect(w)?;
    let raw = sidecars
        .get("selector.intent")
        .ok_or("selector original intent absent")?;
    let intent = wire::parse(raw)?;
    schema(
        &intent,
        "photara.storage.admission-selector",
        1,
        &["token", "original", "old_head_sha256", "next_head_sha256"],
    )?;
    let (o, _) = selected_original(w)?;
    let from = d
        .worlds
        .iter()
        .position(|v| hash(&encode(&v.head)) == intent["old_head_sha256"])
        .ok_or("selector original predecessor")?;
    ensure(
        from < 6 && (current == from || current == from + 1),
        "selector finite adjacent stage",
    )?;
    let next = &d.worlds[from + 1];
    let exact = obj(
        "photara.storage.admission-selector",
        json!({"token":o["token"],"original":reference(&o),"old_head_sha256":hash(&encode(&d.worlds[from].head)),"next_head_sha256":hash(&encode(&next.head))}),
    );
    ensure(intent == exact, "selector exact original transition")?;
    let mut expected = BTreeMap::from([
        ("selector.intent".to_owned(), encode(&exact)),
        ("candidate.commit".to_owned(), encode(&next.commit)),
    ]);
    if sidecars.contains_key("HEAD.next") {
        expected.insert("HEAD.next".to_owned(), encode(&next.head));
    }
    for (digest, bytes) in &next.loose {
        expected.insert(format!("control/{digest}"), bytes.clone());
    }
    ensure(
        w.journal == next.journal,
        "selector observed journal prerequisites",
    )?;
    ensure(*sidecars == expected, "selector exact known sidecar set")?;
    for (id, a) in &w.allocations {
        ensure(
            a.bytes == next.allocations[id].bytes && a.witness == next.allocations[id].witness,
            "selector payload prerequisites",
        )?;
    }
    Ok(next.clone())
}
#[allow(
    clippy::too_many_lines,
    reason = "Keep the exact original metadata contract in a single bounded validation pass"
)]
fn original_shape(o: &Value) -> Result<()> {
    schema(
        o,
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
        number(&o["token"])? > 0 && o["kind"] == "graph",
        "original metadata graph token",
    )?;
    let nonce = text(&o["nonce"])?;
    ensure(
        nonce.len() == 64
            && nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "original metadata nonce",
    )?;
    fields(&o["scope"], &["profile", "incarnation", "directory"])?;
    fields(&o["scope"]["directory"], &["device", "inode"])?;
    let ledger = &o["old"]["ledger"];
    ensure(
        o["scope"]["profile"] == ledger["profile"]
            && o["scope"]["incarnation"] == ledger["incarnation"]
            && o["scope"]["directory"] == json!({"device":"7","inode":"99"}),
        "original metadata local scope",
    )?;
    ensure(
        number(&o["reserve"])? == RESERVE
            && number(&o["cleanup_bound"])? == CLEANUP
            && number(&ledger["total_charge"])?
                .checked_add(RESERVE)
                .is_some_and(|n| number(&o["project_limit"]).is_ok_and(|limit| n <= limit)),
        "original metadata reserve capacity",
    )?;
    fields(
        &o["old"],
        &["head", "commit", "envelope", "ledger", "overlay", "states"],
    )?;
    let mut roots = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for row in arr(&o["old"]["states"])? {
        fields(row, &["root", "value"])?;
        state_header(
            &row["value"],
            &o["old"]["commit"]["bootstrap_sha256"],
            &o["old"]["commit"]["root_set"]["library_id"],
        )?;
        ensure(
            row["root"] == reference(&row["value"])
                && roots.insert(key(&row["root"])?)
                && ids.insert(text(&row["value"]["root_id"])?),
            "original metadata retained state identity",
        )?;
    }
    let oldroot = &o["old"]["commit"]["root_set"];
    ensure(
        roots.len() == 3
            && roots.contains(&key(&oldroot["active"])?)
            && roots.contains(&key(&oldroot["recovery"])?),
        "original metadata retained root set",
    )?;
    fields(
        &o["request"],
        &["operation_id", "request_sha256", "intent", "receipt"],
    )?;
    for r in [
        &o["request"]["intent"],
        &o["request"]["receipt"],
        &o["old_hold"],
    ] {
        key(r)?;
    }
    fields(
        &o["payload_recipe"],
        &["codec", "allocations", "target_placement", "base_inventory"],
    )?;
    fields(
        &o["payload_recipe"]["target_placement"],
        &["generation", "root_placements"],
    )?;
    ensure(
        o["payload_recipe"]["target_placement"]["generation"] == "2",
        "original metadata exact target generation",
    )?;
    fields(
        &o["semantic_target"],
        &[
            "active",
            "recovery",
            "pinned_roots",
            "operation_index",
            "conversion_source",
        ],
    )?;
    for v in o["semantic_target"].as_object().unwrap().values() {
        key(v)?;
    }
    fields(
        &o["control_bounds"],
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
        o["control_bounds"]["marker_bytes"] == "0"
            && o["control_bounds"]["newborn_binding_bytes"] == "0"
            && number(&o["control_bounds"]["aggregate_control_bytes"])?
                <= number(&ledger["standing_control"])?,
        "original metadata standing control bound",
    )?;
    Ok(())
}
#[cfg(test)]
pub(super) fn rebind_original_negative(
    w: &wire::World,
    change: impl FnOnce(&mut Value),
) -> Result<wire::World> {
    rebind_evidence_negative(w, |o, _| change(o))
}
#[cfg(test)]
#[allow(
    clippy::many_single_char_names,
    reason = "Negative fixture O/P/F symbols mirror the protocol"
)]
pub(super) fn rebind_evidence_negative(
    w: &wire::World,
    change: impl FnOnce(&mut Value, &mut Value),
) -> Result<wire::World> {
    let (mut o, p) = selected_original(w)?;
    let mut f = w.loose(&p["finalization"])?;
    change(&mut o, &mut f);
    f["original"] = reference(&o);
    f["token"] = o["token"].clone();
    f["target_projection"]["placement"] = o["payload_recipe"]["target_placement"].clone();
    let mut old = w.clone();
    old.head = o["old"]["head"].clone();
    old.commit = o["old"]["commit"].clone();
    old.loose.clear();
    for name in ["ledger", "envelope", "overlay"] {
        let v = &o["old"][name];
        old.loose.insert(hash(&encode(v)), encode(v));
    }
    let env = w.loose(&w.commit["root_set"]["placement"]["accounting"])?;
    let ledger = w.loose(&env["ledger"])?;
    let prepared = prepare::Prepared {
        authored: BTreeMap::new(),
        intent: w.loose(&o["request"]["intent"])?,
        receipt: f["prepared_receipt"].clone(),
    };
    let pool = decoder::Plan {
        files: w
            .allocations
            .iter()
            .map(|(id, a)| (id.clone(), a.bytes.clone()))
            .collect(),
        target: Value::Null,
        base_inventory: Value::Null,
        root_placements: Value::Null,
    };
    let count = number(&packed_value(&pool, &o["payload_recipe"]["base_inventory"])?["count"])?;
    let stage = STAGES.iter().position(|s| p["stage"] == *s).unwrap();
    let mut next = control_chain(
        &old,
        &o,
        &prepared,
        &f,
        &ledger,
        &f["target_projection"]["active_state"],
        count,
    )?
    .remove(stage);
    next.allocations.clone_from(&w.allocations);
    next.journal.clone_from(&w.journal);
    Ok(next)
}
fn uuid(v: &Value) -> Result<()> {
    let s = text(v)?;
    let id = uuid::Uuid::parse_str(s).map_err(|_| "phase canonical UUID")?;
    ensure(!id.is_nil() && id.to_string() == s, "phase canonical UUID")
}
fn digest(v: &Value) -> Result<()> {
    let s = text(v)?;
    ensure(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "phase canonical digest",
    )
}
fn state_header(v: &Value, bootstrap: &Value, library: &Value) -> Result<()> {
    schema(
        v,
        "photara.package.state-root",
        2,
        &[
            "library_id",
            "bootstrap_sha256",
            "root_id",
            "authored_revision",
            "authored",
            "history",
            "resource_state",
            "operation_index",
            "accepted",
            "journal_inclusion",
            "predecessor",
            "inventory",
        ],
    )?;
    ensure(
        v["library_id"] == *library && v["bootstrap_sha256"] == *bootstrap,
        "phase embedded state package identity",
    )?;
    uuid(&v["root_id"])?;
    number(&v["authored_revision"])?;
    for field in [
        "authored",
        "history",
        "resource_state",
        "operation_index",
        "inventory",
    ] {
        key(&v[field])?;
    }
    fields(&v["accepted"], &["through_ordinal", "prefix_sha256"])?;
    number(&v["accepted"]["through_ordinal"])?;
    digest(&v["accepted"]["prefix_sha256"])?;
    let j = &v["journal_inclusion"];
    fields(
        j,
        &[
            "journal_id",
            "through_sequence",
            "prefix_sha256",
            "resulting_authored_revision",
            "resulting_authored_sha256",
        ],
    )?;
    uuid(&j["journal_id"])?;
    for n in ["through_sequence", "resulting_authored_revision"] {
        number(&j[n])?;
    }
    for h in ["prefix_sha256", "resulting_authored_sha256"] {
        digest(&j[h])?;
    }
    if !v["predecessor"].is_null() {
        fields(&v["predecessor"], &["root_sha256", "authored_revision"])?;
        digest(&v["predecessor"]["root_sha256"])?;
        number(&v["predecessor"]["authored_revision"])?;
    }
    Ok(())
}
