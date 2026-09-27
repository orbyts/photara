//! Explicit admission-only successor. Never executes packed writes or accepts the prepared receipt.
use super::{decoder, prepare, wire};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use wire::{Result, encode, ensure, fields, hash, key, number, reference, schema, text};
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("admission array")
}
fn uuid(v: &Value) -> Result<()> {
    let s = text(v)?;
    let id = uuid::Uuid::parse_str(s).map_err(|_| "admission UUID")?;
    ensure(
        !id.is_nil() && id.to_string() == s,
        "admission nonnil canonical UUID",
    )
}
fn rounded(n: u64) -> Result<u64> {
    n.checked_add(4095)
        .map(|v| v / 4096 * 4096)
        .ok_or("admission rounding overflow")
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or("admission arithmetic overflow")
}
fn count_frames(bytes: &[u8]) -> Result<u64> {
    let mut p = 0usize;
    let mut count = 0u64;
    while p < bytes.len() {
        ensure(
            bytes.len() - p >= 16 && &bytes[p..p + 8] == b"PS2PKD01",
            "planned complete frame",
        )?;
        ensure(
            bytes[p + 8] <= 3 && bytes[p + 9..p + 12] == [0, 0, 0],
            "planned frame dispatch",
        )?;
        let n = usize::try_from(u32::from_le_bytes(
            bytes[p + 12..p + 16].try_into().unwrap(),
        ))
        .map_err(|_| "planned length")?;
        let end = p
            .checked_add(16)
            .and_then(|v| v.checked_add(n))
            .ok_or("planned frame overflow")?;
        ensure(end <= bytes.len(), "planned frame end")?;
        if bytes[p + 8] == 0 {
            ensure(
                bytes[p + 16..end].iter().all(|b| *b == 0),
                "planned canonical padding",
            )?;
        } else {
            wire::parse(&bytes[p + 16..end])?;
        }
        count = add(count, 1)?;
        p = end;
    }
    Ok(count)
}
pub(super) fn load_selected(c: &Value, base: &Value) -> Result<wire::World> {
    let mut selected = c["selected"].clone();
    for name in ["allocations", "source_files", "source_witnesses"] {
        selected[name] = base[name].clone();
    }
    wire::World::load_transition(&selected)
}
pub(super) struct Proof {
    pub(super) reserved: u64,
    pub(super) worst_growth: u64,
    pub(super) control_peak: u64,
    pub(super) global: BTreeSet<wire::Key>,
}
/// Full prospective closure must already have passed the typed transition reader.
/// This routine proves actual admission controls and unchanged physical state separately.
#[allow(
    clippy::too_many_lines,
    reason = "Keep actual selected control validation in authority order"
)]
pub(super) fn verify(
    c: &Value,
    old: &wire::World,
    next: &wire::World,
    future: &wire::World,
    oldproof: &wire::Proof,
    prepared: &prepare::Prepared,
    futureproof: &wire::Proof,
) -> Result<Proof> {
    // The proposed future is sizing data only, not the actual admitted package.
    ensure(
        next.allocations.len() == old.allocations.len()
            && next.allocations.iter().all(|(id, a)| {
                old.allocations.get(id).is_some_and(|v| {
                    v.bytes == a.bytes && v.arena == a.arena && v.witness == a.witness
                })
            })
            && next.source == old.source
            && next.source_witnesses == old.source_witnesses
            && next.journal == old.journal,
        "admission no packed/source/journal effect",
    )?;
    ensure(
        next.manifest == old.manifest,
        "admission unchanged bootstrap",
    )?;
    let root = &next.commit["root_set"];
    let oldroot = &old.commit["root_set"];
    let envelope = next.loose(&root["placement"]["accounting"])?;
    schema(
        &envelope,
        "photara.storage.accounting-envelope",
        1,
        &["ledger", "holds"],
    )?;
    let oldenvelope = old.loose(&oldroot["placement"]["accounting"])?;
    let ledger = next.loose(&envelope["ledger"])?;
    let oldledger = old.loose(&oldenvelope["ledger"])?;
    ensure(
        ledger == oldledger,
        "admission unchanged exact measured ledger",
    )?;
    let holds = next.loose(&envelope["holds"])?;
    schema(
        &holds,
        "photara.storage.hold-leaf",
        1,
        &["count", "reserved", "consumed", "remaining", "entries"],
    )?;
    ensure(
        holds["count"] == "1" && array(&holds["entries"])?.len() == 1,
        "bounded single original hold",
    )?;
    let entry = &holds["entries"][0];
    fields(entry, &["token", "original", "phase"])?;
    let original = next.loose(&entry["original"])?;
    schema(
        &original,
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
    let phase = next.loose(&entry["phase"])?;
    schema(
        &phase,
        "photara.storage.operation-phase",
        1,
        &[
            "token",
            "original",
            "stage",
            "consumed",
            "remaining",
            "newborn_binding",
            "finalization",
            "cleanup",
        ],
    )?;
    let reserve = number(&original["reserve"])?;
    let cleanup = number(&original["cleanup_bound"])?;
    ensure(
        number(&original["token"])? != 0
            && entry["token"] == original["token"]
            && phase["token"] == original["token"]
            && phase["original"] == entry["original"]
            && original["kind"] == "graph"
            && original["original_codec"] == "photara.codec.ps2-same-tip-admission-v1",
        "exact original admission dispatch",
    )?;
    ensure(
        phase["stage"] == "admitted"
            && phase["consumed"] == "0"
            && number(&phase["remaining"])? == reserve
            && phase["newborn_binding"].is_null()
            && phase["finalization"].is_null()
            && phase["cleanup"].is_null()
            && original["generation_plan"].is_null()
            && original["retention_intent"].is_null(),
        "admission-only phase/null dispatch",
    )?;
    ensure(
        number(&holds["reserved"])? == reserve
            && holds["consumed"] == "0"
            && number(&holds["remaining"])? == reserve,
        "derived hold summaries",
    )?;
    ensure(
        cleanup > 0 && add(oldproof.total, reserve)? <= number(&original["project_limit"])?,
        "original full reserve capacity before effects",
    )?;
    fields(&original["scope"], &["profile", "incarnation", "directory"])?;
    fields(&original["scope"]["directory"], &["device", "inode"])?;
    ensure(
        original["scope"]["profile"] == oldledger["profile"]
            && original["scope"]["incarnation"] == oldledger["incarnation"]
            && original["scope"]["directory"] == json!({"device":"7","inode":"99"}),
        "original local fixture scope",
    )?;
    let nonce = text(&original["nonce"])?;
    ensure(
        nonce.len() == 64
            && nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "original nonce bytes",
    )?;
    let oldoverlay = old.loose(&oldroot["inventory"])?;
    let mut states = oldproof
        .roles
        .values()
        .map(|p| json!({"root":reference(&p.state),"value":p.state}))
        .collect::<Vec<_>>();
    states.sort_by_key(|s| key(&s["root"]).unwrap());
    states.dedup();
    ensure(
        original["old"]
            == json!({"head":old.head,"commit":old.commit,"envelope":oldenvelope,"ledger":oldledger,"overlay":oldoverlay,"states":states})
            && original["old_hold"] == oldenvelope["holds"],
        "exact authenticated original reconstruction controls",
    )?;
    ensure(
        original["request"]
            == json!({"operation_id":prepared.intent["operation_id"],"request_sha256":prepared.receipt["request_sha256"],"intent":reference(&prepared.intent),"receipt":reference(&prepared.receipt)}),
        "actual Core-prepared request",
    )?;
    ensure(
        next.loose(&original["request"]["intent"])? == prepared.intent
            && next.loose(&original["request"]["receipt"])? == prepared.receipt,
        "exact prepared loose bytes",
    )?;
    let overlay = next.loose(&root["inventory"])?;
    schema(
        &overlay,
        "photara.package.inventory-overlay",
        1,
        &["base", "controls", "count"],
    )?;
    let mut expectedroot = oldroot.clone();
    expectedroot["inventory"] = reference(&overlay);
    expectedroot["placement"]["accounting"] = reference(&envelope);
    ensure(
        *root == expectedroot,
        "actual admitted RootSet exact unchanged selectors",
    )?;
    let mut expectedcommit = old.commit.clone();
    for n in ["commit_id", "write_id"] {
        uuid(&next.commit[n])?;
        expectedcommit[n] = next.commit[n].clone();
    }
    ensure(
        next.commit["commit_id"] != old.commit["commit_id"]
            && next.commit["write_id"] != old.commit["write_id"],
        "fresh successor control IDs",
    )?;
    expectedcommit["package_revision"] =
        json!(add(number(&old.commit["package_revision"])?, 1)?.to_string());
    expectedcommit["parent"] =
        json!({"commit_id":old.commit["commit_id"],"commit_sha256":hash(&encode(&old.commit))});
    expectedcommit["root_set"] = root.clone();
    ensure(
        next.commit == expectedcommit,
        "actual HEAD exact successor commit",
    )?;
    let mut expectedhead = old.head.clone();
    expectedhead["commit_id"] = next.commit["commit_id"].clone();
    expectedhead["commit_sha256"] = json!(hash(&encode(&next.commit)));
    ensure(next.head == expectedhead, "actual selected HEAD hash")?;
    let controls = [
        ledger.clone(),
        envelope.clone(),
        original.clone(),
        phase,
        holds,
        prepared.intent.clone(),
        prepared.receipt.clone(),
    ];
    let mut refs = controls.iter().map(reference).collect::<Vec<_>>();
    refs.sort_by_key(|r| key(r).unwrap());
    ensure(
        overlay["controls"] == json!(refs) && overlay["base"] == oldoverlay["base"],
        "actual exact control overlay",
    )?;
    let mut global = oldproof.global.clone();
    for r in array(&oldoverlay["controls"])? {
        global.remove(&key(r)?);
    }
    for r in &refs {
        ensure(global.insert(key(r)?), "new controls disjoint packed base")?;
    }
    ensure(
        number(&overlay["count"])? == global.len() as u64,
        "actual complete successor global count",
    )?;
    let expectedkeys = controls
        .iter()
        .chain([&overlay])
        .map(|v| hash(&encode(v)))
        .collect::<BTreeSet<_>>();
    ensure(
        next.loose.keys().cloned().collect::<BTreeSet<_>>() == expectedkeys,
        "exact live loose keep set",
    )?;
    let regenerated = decoder::regenerate(old, oldproof, prepared)?;
    ensure(
        regenerated.files.len() == future.allocations.len()
            && regenerated
                .files
                .iter()
                .all(|(id, b)| future.allocations.get(id).is_some_and(|a| a.bytes == *b)),
        "independent deterministic full append regeneration",
    )?;
    ensure(
        regenerated.target == original["semantic_target"]
            && regenerated.base_inventory == original["payload_recipe"]["base_inventory"]
            && original["payload_recipe"]["target_placement"]["root_placements"]
                == regenerated.root_placements,
        "independent deterministic finalizer target regeneration",
    )?;
    let mut expected_receipts = oldproof.roles["active"].receipts.clone();
    expected_receipts.push(prepared.receipt.clone());
    ensure(
        futureproof.roles["active"].receipts == expected_receipts
            && futureproof.roles["active"].state["authored"]["sha256"]
                == prepared.receipt["after"]["digest"]
            && futureproof.roles["active"].state["authored_revision"]
                == prepared.receipt["after"]["revision"]
            && future.source == old.source
            && future.source_witnesses == old.source_witnesses,
        "exact prepared next receipt and original source preservation",
    )?;
    // Full independently parsed prospective layout is committed, never installed here.
    let target = &original["semantic_target"];
    fields(
        target,
        &[
            "active",
            "recovery",
            "pinned_roots",
            "operation_index",
            "conversion_source",
        ],
    )?;
    for field in [
        "active",
        "recovery",
        "pinned_roots",
        "operation_index",
        "conversion_source",
    ] {
        ensure(
            target[field] == future.commit["root_set"][field],
            "typed prospective semantic target",
        )?;
    }
    ensure(
        target["recovery"] == oldroot["active"]
            && target["pinned_roots"] == oldroot["pinned_roots"]
            && target["conversion_source"] == oldroot["conversion_source"],
        "exact recovery rotation and retained roots",
    )?;
    let recipe = &original["payload_recipe"];
    fields(
        recipe,
        &["codec", "allocations", "target_placement", "base_inventory"],
    )?;
    ensure(
        recipe["codec"] == "photara.codec.ps2-same-tip-layout-v1"
            && recipe["allocations"] == c["dry_run"]["recipe"],
        "original deterministic layout recipe",
    )?;
    let future_root = &future.commit["root_set"];
    let future_envelope = future.loose(&future_root["placement"]["accounting"])?;
    let future_ledger = future.loose(&future_envelope["ledger"])?;
    let future_overlay = future.loose(&future_root["inventory"])?;
    ensure(
        recipe["target_placement"]
            == json!({"generation":future_root["placement"]["generation"],"root_placements":future_root["placement"]["root_placements"]})
            && recipe["base_inventory"] == future_overlay["base"],
        "full finalizer physical target commitment",
    )?;
    let mut ids = BTreeSet::new();
    let mut prior = None;
    let mut growth = 0u64;
    for r in array(&recipe["allocations"])? {
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
        let id = text(&r["allocation_id"])?;
        ensure(
            prior.is_none_or(|p| p < id) && ids.insert(id.to_owned()),
            "sorted exact append plan",
        )?;
        prior = Some(id);
        let before = old
            .allocations
            .get(id)
            .ok_or("original planned allocation")?;
        let after = future
            .allocations
            .get(id)
            .ok_or("future planned allocation")?;
        let end = usize::try_from(number(&r["original_end"])?).map_err(|_| "original end")?;
        ensure(
            end == before.bytes.len()
                && after.bytes.starts_with(&before.bytes)
                && r["original_sha256"] == hash(&before.bytes)
                && r["witness"] == before.witness
                && after.witness == before.witness
                && r["arena"] == before.arena
                && after.arena == before.arena
                && number(&r["final_end"])? == after.bytes.len() as u64,
            "full original prefix and witnessed append",
        )?;
        ensure(
            r["framed_sha256"] == hash(&after.bytes[end..])
                && number(&r["frame_count"])? == count_frames(&after.bytes[end..])?,
            "full planned raw suffix including unreachable frames",
        )?;
        let tip = array(&oldledger["tips"])?
            .iter()
            .find(|t| t["allocation_id"] == id)
            .ok_or("planned original tip")?;
        growth = add(
            growth,
            rounded(after.bytes.len() as u64)?.saturating_sub(number(&tip["registered_charge"])?),
        )?;
    }
    ensure(
        ids == array(&oldledger["tips"])?
            .iter()
            .map(|t| text(&t["allocation_id"]).map(str::to_owned))
            .collect::<Result<BTreeSet<_>>>()?
            && add(growth, cleanup)? <= reserve,
        "whole finite finalizer corridor reserved",
    )?;
    ensure(
        futureproof.total == add(oldproof.total, growth)?
            && future_ledger["sealed_charge_root"] == oldledger["sealed_charge_root"]
            && future_ledger["retained_file_charge_root"] == oldledger["retained_file_charge_root"],
        "prospective growth excludes recharged sealed/source allocations",
    )?;
    let bounds = &original["control_bounds"];
    fields(
        bounds,
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
        bounds["marker_bytes"] == "0"
            && bounds["newborn_binding_bytes"] == "0"
            && number(&bounds["aggregate_control_bytes"])?
                <= number(&oldledger["standing_control"])?,
        "same-tip control bound dispatch",
    )?;
    // Original and candidate immutable controls coexist; separate HEAD/commit plus
    // staged selector, bounded intent and directory occupy three additional blocks.
    let mut files = BTreeMap::new();
    for w in [old, next] {
        for (k, b) in &w.loose {
            files.insert(k.clone(), b.len() as u64);
        }
        for v in [&w.manifest, &w.head, &w.commit] {
            files.insert(hash(&encode(v)), encode(v).len() as u64);
        }
    }
    let selector_intent = json!({"schema":{"id":"photara.storage.admission-selector","version":1},
        "project_id":wire::PROJECT,"extensions":{},"token":original["token"],"original":reference(&original),
        "old_head_sha256":hash(&encode(&old.head)),"next_head_sha256":hash(&encode(&next.head))});
    let staged_head = encode(&next.head);
    ensure(
        encode(&selector_intent).len() <= 4096 && staged_head.len() <= 4096,
        "actual bounded admission intent and staged HEAD",
    )?;
    let dispatch = add(
        add(
            rounded(encode(&selector_intent).len() as u64)?,
            rounded(staged_head.len() as u64)?,
        )?,
        4096,
    )?;
    let mut peak = dispatch;
    for len in files.values() {
        peak = add(peak, rounded(*len)?)?;
    }
    ensure(
        peak <= number(&bounds["aggregate_control_bytes"])?
            && files.len() as u64 + 3 <= number(&bounds["aggregate_control_count"])?,
        "simultaneous original admission controls bounded",
    )?;
    // Sizing-only sentinels are never selected observations or charges. They cover
    // the exact finite future control shapes with maximum-width decimal fields.
    let maximum = u64::MAX.to_string();
    let mut final_ledger = future_ledger.clone();
    final_ledger["total_charge"] = json!(maximum);
    for tip in final_ledger["tips"].as_array_mut().ok_or("sizing tips")? {
        tip["registered_charge"] = json!(maximum);
        tip["observation"]["charged_high_water"] = json!(maximum);
    }
    let writes = array(&recipe["allocations"])?
        .iter()
        .map(|r| {
            let mut w = r.clone();
            w["recipe_sha256"] = json!(hash(&encode(r)));
            w
        })
        .collect::<Vec<_>>();
    let finalizer = json!({"schema":{"id":"photara.storage.finalization-control","version":1},
        "project_id":wire::PROJECT,"extensions":{},"token":original["token"],
        "generation_plan":null,"newborn_binding":null,"payload_completion":recipe["allocations"],
        "sealed_allocations":[],"recipe_codec":recipe["codec"],"writes":writes,
        "target_projection":{"active":target["active"],"recovery":target["recovery"],
            "pinned_roots":target["pinned_roots"],"operation_index":target["operation_index"],
            "conversion_source":target["conversion_source"],"retention_evidence":future_root["retention_evidence"],
            "base_inventory":recipe["base_inventory"],"placement":recipe["target_placement"]}});
    ensure(
        encode(&finalizer).len() as u64 <= number(&bounds["finalization_bytes"])?,
        "finite actual finalizer control sizing",
    )?;
    let future_phase = json!({"schema":{"id":"photara.storage.operation-phase","version":1},
        "project_id":wire::PROJECT,"extensions":{},"token":original["token"],"original":reference(&original),
        "stage":"finalizer-selected","consumed":maximum,"remaining":maximum,
        "newborn_binding":null,"finalization":reference(&finalizer),
        "cleanup":{"remaining_roles":[],"directory_barrier":true}});
    ensure(
        encode(&future_phase).len() as u64 <= number(&bounds["phase_bytes"])?,
        "maximum-width phase control sizing",
    )?;
    let future_holds = json!({"schema":{"id":"photara.storage.hold-leaf","version":1},"project_id":wire::PROJECT,"extensions":{},
        "count":"1","reserved":original["reserve"],"consumed":maximum,"remaining":maximum,
        "entries":[{"token":original["token"],"original":reference(&original),"phase":reference(&future_phase)}]});
    let future_envelope = json!({"schema":{"id":"photara.storage.accounting-envelope","version":1},"project_id":wire::PROJECT,"extensions":{},
        "ledger":reference(&final_ledger),"holds":reference(&future_holds)});
    let future_controls = [
        &final_ledger,
        &future_envelope,
        &original,
        &future_phase,
        &future_holds,
        &prepared.intent,
        &prepared.receipt,
        &finalizer,
    ];
    let mut future_refs = future_controls
        .iter()
        .map(|v| reference(v))
        .collect::<Vec<_>>();
    future_refs.sort_by_key(|r| key(r).unwrap());
    let future_overlay = json!({"schema":{"id":"photara.package.inventory-overlay","version":1},"project_id":wire::PROJECT,"extensions":{},
        "base":recipe["base_inventory"],"controls":future_refs,"count":maximum});
    let mut future_files = BTreeMap::new();
    for (k, b) in &next.loose {
        future_files.insert(k.clone(), b.len() as u64);
    }
    for v in future_controls.into_iter().chain([
        &future_overlay,
        &future.manifest,
        &future.head,
        &future.commit,
        &next.head,
        &next.commit,
    ]) {
        future_files.insert(hash(&encode(v)), encode(v).len() as u64);
    }
    let mut future_intent = selector_intent;
    future_intent["old_head_sha256"] = json!(hash(&encode(&next.head)));
    future_intent["next_head_sha256"] = json!(hash(&encode(&future.head)));
    ensure(
        encode(&future_intent).len() <= 4096 && encode(&future.head).len() <= 4096,
        "actual bounded future selector roles",
    )?;
    let mut future_peak = add(
        add(
            rounded(encode(&future_intent).len() as u64)?,
            rounded(encode(&future.head).len() as u64)?,
        )?,
        4096,
    )?;
    for len in future_files.values() {
        future_peak = add(future_peak, rounded(*len)?)?;
    }
    ensure(
        future_peak <= number(&bounds["aggregate_control_bytes"])?
            && future_files.len() as u64 + 3 <= number(&bounds["aggregate_control_count"])?,
        "complete future control coexistence sizing",
    )?;
    peak = peak.max(future_peak);
    Ok(Proof {
        reserved: reserve,
        worst_growth: growth,
        control_peak: peak,
        global,
    })
}
