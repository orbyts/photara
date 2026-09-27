//! Pure finite selected-route contracts; physical reads and selector resolution remain caller duties.
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Result<T> = std::result::Result<T, &'static str>;
const PROFILE: &str = "example.ps2.synthetic-local-profile-v1";
const INCARNATION: &str = "71000000-0000-4000-8000-000000000001";
fn ensure(ok: bool, reason: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(reason) }
}
fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let m = v.as_object().ok_or("contract object")?;
    ensure(
        m.len() == names.len() && names.iter().all(|n| m.contains_key(*n)),
        "contract exact fields",
    )
}
fn number(v: &Value) -> Result<u64> {
    let s = v.as_str().ok_or("contract decimal")?;
    let n = s.parse::<u64>().map_err(|_| "contract decimal range")?;
    ensure(n.to_string() == s, "contract canonical decimal")?;
    Ok(n)
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or("contract string")
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("contract array")
}
fn bytes(v: &Value) -> Result<Vec<u8>> {
    photara_core::canonical_json(v).map_err(|_| "contract canonical bytes")
}
fn hash(v: &Value) -> Result<String> {
    Ok(super::packed::hash(&bytes(v)?))
}
fn reference(v: &Value) -> Result<Value> {
    Ok(json!({"kind":"json","sha256":hash(v)?,"byte_length":bytes(v)?.len().to_string()}))
}
fn digest(v: &Value) -> Result<()> {
    let s = text(v)?;
    ensure(
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "contract digest",
    )
}
fn id(v: &Value) -> Result<()> {
    let s = text(v)?;
    let u = uuid::Uuid::parse_str(s).map_err(|_| "contract UUID")?;
    ensure(
        !u.is_nil() && u.to_string() == s,
        "contract canonical nonnil UUID",
    )
}
fn object_ref(v: &Value) -> Result<()> {
    fields(v, &["kind", "sha256", "byte_length"])?;
    ensure(
        v["kind"] == "json" && number(&v["byte_length"])? > 0,
        "contract ObjectRef",
    )?;
    digest(&v["sha256"])
}
fn schema(v: &Value, name: &str, extra: &[&str]) -> Result<()> {
    let mut names = vec!["schema", "project_id", "extensions"];
    names.extend_from_slice(extra);
    fields(v, &names)?;
    ensure(
        v["schema"] == json!({"id":format!("example.ps2.{name}"),"version":1})
            && v["project_id"] == "10000000-0000-4000-8000-000000000001",
        "contract schema/project",
    )?;
    ensure(
        v["extensions"].as_object().is_some_and(|m| {
            m.keys()
                .all(|k| photara_core::contracts::schema::QualifiedName::parse(k.clone()).is_ok())
        }),
        "contract extension namespace",
    )
}
fn witness(v: &Value, allocation: &Value) -> Result<()> {
    fields(
        v,
        &["profile", "incarnation", "allocation_id", "device", "inode"],
    )?;
    id(allocation)?;
    ensure(
        v["profile"] == PROFILE
            && v["incarnation"] == INCARNATION
            && v["allocation_id"] == *allocation,
        "contract scoped witness",
    )?;
    number(&v["device"])?;
    ensure(number(&v["inode"])? > 0, "contract nonzero inode")
}
fn tips(core: &Value) -> Result<BTreeMap<String, &Value>> {
    let mut out = BTreeMap::new();
    for tip in array(&core["tips"])? {
        fields(tip, &["witness", "arena", "extent", "registered_charge"])?;
        let allocation = &tip["witness"]["allocation_id"];
        witness(&tip["witness"], allocation)?;
        ensure(
            matches!(text(&tip["arena"])?, "data" | "metadata"),
            "contract arena",
        )?;
        ensure(
            number(&tip["registered_charge"])? >= number(&tip["extent"])?,
            "contract attributed tip",
        )?;
        ensure(
            out.insert(text(allocation)?.to_owned(), tip).is_none(),
            "contract duplicate tip",
        )?;
    }
    ensure(!out.is_empty() && out.len() <= 16, "contract bounded tips")?;
    Ok(out)
}
fn physical(v: &Value) -> Result<()> {
    fields(
        v,
        &[
            "allocation_id",
            "arena",
            "offset",
            "byte_length",
            "record_sha256",
        ],
    )?;
    id(&v["allocation_id"])?;
    ensure(
        matches!(text(&v["arena"])?, "data" | "metadata") && number(&v["byte_length"])? > 0,
        "contract physical range",
    )?;
    number(&v["offset"])?
        .checked_add(number(&v["byte_length"])?)
        .ok_or("contract physical overflow")?;
    digest(&v["record_sha256"])
}
#[allow(
    clippy::too_many_lines,
    reason = "Validate immutable admission before any phase interpretation"
)]
pub(super) fn validate_original(o: &Value) -> Result<()> {
    schema(
        o,
        "route-original",
        &[
            "token",
            "kind",
            "scope",
            "old",
            "target",
            "recipe",
            "reserve",
            "project_limit",
            "cleanup_bound",
            "maximum_control_bytes",
            "maximum_control_count",
            "operation",
        ],
    )?;
    ensure(
        number(&o["token"])? > 0 && matches!(text(&o["kind"])?, "graph" | "retire"),
        "contract original token/kind",
    )?;
    fields(&o["scope"], &["profile", "incarnation", "codec"])?;
    ensure(
        o["scope"]
            == json!({"profile":PROFILE,"incarnation":INCARNATION,"codec":"example.ps2.route-v1"}),
        "contract original scope/codec",
    )?;
    fields(
        &o["old"],
        &["head", "commit", "ledger", "envelope", "overlay", "states"],
    )?;
    let old_root = &o["old"]["commit"]["root_set"];
    fields(&o["old"]["states"], &["active", "recovery"])?;
    for role in ["active", "recovery"] {
        ensure(
            o["old"]["states"][role].is_object()
                && reference(&o["old"]["states"][role])? == old_root[role],
            "contract original state reconstruction",
        )?;
    }
    let old_envelope = &o["old"]["envelope"];
    let old_overlay = &o["old"]["overlay"];
    if old_envelope.is_null() {
        ensure(
            old_overlay.is_null()
                && old_root["placement"]["accounting"] == reference(&o["old"]["ledger"])?,
            "contract original direct ledger",
        )?;
    } else {
        ensure(
            !old_overlay.is_null()
                && old_root["placement"]["accounting"] == reference(old_envelope)?
                && old_envelope["ledger"] == reference(&o["old"]["ledger"])?
                && old_root["inventory"] == reference(old_overlay)?,
            "contract original embedded control reconstruction",
        )?;
    }
    ensure(
        o["old"]["head"]["commit_sha256"] == hash(&o["old"]["commit"])?
            && o["old"]["head"]["commit_id"] == o["old"]["commit"]["commit_id"],
        "contract original exact commit",
    )?;
    let old = &o["old"]["ledger"];
    let old_tips = tips(old)?;
    ensure(
        old["profile"] == PROFILE && old["incarnation"] == INCARNATION,
        "contract original ledger scope",
    )?;
    number(&old["total_charge"])?;
    fields(
        &o["target"],
        &[
            "active",
            "recovery",
            "operation_index",
            "conversion_source",
            "base_inventory",
            "placement",
        ],
    )?;
    for field in ["active", "recovery", "operation_index", "base_inventory"] {
        object_ref(&o["target"][field])?;
    }
    if !o["target"]["conversion_source"].is_null() {
        object_ref(&o["target"]["conversion_source"])?;
    }
    let placement = &o["target"]["placement"];
    fields(
        placement,
        &[
            "generation",
            "active_locator",
            "recovery_locator",
            "active_ownership",
            "recovery_ownership",
        ],
    )?;
    number(&placement["generation"])?;
    for field in [
        "active_locator",
        "recovery_locator",
        "active_ownership",
        "recovery_ownership",
    ] {
        physical(&placement[field])?;
    }
    fields(&o["recipe"], &["codec", "allocations", "source"])?;
    ensure(
        o["recipe"]["codec"] == "example.ps2.closed-layout-v1",
        "contract recipe codec",
    )?;
    let allocations = array(&o["recipe"]["allocations"])?;
    ensure(
        !allocations.is_empty() && allocations.len() <= old_tips.len(),
        "contract bounded recipe",
    )?;
    let mut seen = BTreeSet::new();
    let mut planned_growth = 0u64;
    for a in allocations {
        fields(
            a,
            &[
                "allocation_id",
                "arena",
                "witness",
                "old_end",
                "old_sha256",
                "end",
                "sha256",
                "append_sha256",
                "frame_count",
            ],
        )?;
        let allocation = text(&a["allocation_id"])?;
        ensure(
            seen.insert(allocation),
            "contract duplicate recipe allocation",
        )?;
        let old_tip = old_tips
            .get(allocation)
            .ok_or("contract unknown recipe allocation")?;
        witness(&a["witness"], &a["allocation_id"])?;
        ensure(
            a["witness"] == old_tip["witness"]
                && a["arena"] == old_tip["arena"]
                && number(&a["old_end"])? == number(&old_tip["extent"])?
                && number(&a["end"])? >= number(&a["old_end"])?,
            "contract original tip range/witness",
        )?;
        for field in ["old_sha256", "sha256", "append_sha256"] {
            digest(&a[field])?;
        }
        let rounded = number(&a["end"])?
            .checked_add(4095)
            .and_then(|n| (n / 4096).checked_mul(4096))
            .ok_or("contract planned allocation overflow")?;
        let delta = rounded.saturating_sub(number(&old_tip["registered_charge"])?);
        planned_growth = planned_growth
            .checked_add(delta)
            .ok_or("contract planned growth overflow")?;
        let added = number(&a["end"])? - number(&a["old_end"])?;
        let frames = number(&a["frame_count"])?;
        ensure(
            (added == 0) == (frames == 0) && frames.checked_mul(16).is_some_and(|n| n <= added),
            "contract framed suffix bound",
        )?;
    }
    if o["kind"] == "retire" {
        let s = &o["recipe"]["source"];
        fields(
            s,
            &[
                "allocation_id",
                "witness",
                "byte_length",
                "sha256",
                "registered_charge",
                "charge",
                "charge_path",
            ],
        )?;
        ensure(s["charge"].is_object(), "contract source charge object")?;
        let path = array(&s["charge_path"])?;
        ensure(
            !path.is_empty() && path.len() <= 256 && path.iter().all(Value::is_object),
            "contract bounded original charge proof",
        )?;
        witness(&s["witness"], &s["allocation_id"])?;
        digest(&s["sha256"])?;
        ensure(
            number(&s["byte_length"])? > 0
                && number(&s["registered_charge"])? >= number(&s["byte_length"])?
                && !old_tips.contains_key(text(&s["allocation_id"])?),
            "contract sealed source charge",
        )?;
        ensure(o["operation"].is_null(), "contract retirement operation")?;
    } else {
        ensure(
            o["recipe"]["source"].is_null(),
            "contract graph no retirement source",
        )?;
        fields(&o["operation"], &["intent", "receipt"])?;
        object_ref(&o["operation"]["intent"])?;
        object_ref(&o["operation"]["receipt"])?;
    }
    let reserve = number(&o["reserve"])?;
    ensure(
        number(&old["total_charge"])?
            .checked_add(reserve)
            .is_some_and(|n| n <= number(&o["project_limit"]).unwrap_or(0)),
        "contract fixed project capacity before effects",
    )?;
    let cleanup = number(&o["cleanup_bound"])?;
    let cap = number(&o["maximum_control_bytes"])?;
    let count = number(&o["maximum_control_count"])?;
    ensure(
        cleanup > 0
            && planned_growth
                .checked_add(cleanup)
                .is_some_and(|n| n <= reserve)
            && cap > 0
            && (1..=64).contains(&count)
            && cap <= number(&old["standing_control"])?
            && bytes(o)?.len() as u64 <= cap,
        "contract original finite control/reserve",
    )
}
fn consumption(o: &Value, p: &Value) -> Result<u64> {
    let old = tips(&o["old"]["ledger"])?;
    let observations = array(&p["observations"])?;
    ensure(
        observations.len() == old.len(),
        "contract exact observation coverage",
    )?;
    let recipe = array(&o["recipe"]["allocations"])?;
    let mut seen = BTreeSet::new();
    let mut c = 0u64;
    for row in observations {
        fields(
            row,
            &["allocation_id", "witness", "extent", "registered_charge"],
        )?;
        let allocation = text(&row["allocation_id"])?;
        ensure(seen.insert(allocation), "contract duplicate observation")?;
        let old_tip = old
            .get(allocation)
            .ok_or("contract unknown observed allocation")?;
        let end = recipe
            .iter()
            .find(|a| a["allocation_id"] == row["allocation_id"])
            .map_or(&old_tip["extent"], |a| &a["end"]);
        ensure(
            row["witness"] == old_tip["witness"] && row["extent"] == *end,
            "contract observed identity/exact final extent",
        )?;
        let charge = number(&row["registered_charge"])?;
        let prior = number(&old_tip["registered_charge"])?;
        ensure(
            charge >= prior && charge >= number(end)?,
            "contract no highwater decrease",
        )?;
        c = c
            .checked_add(charge - prior)
            .ok_or("contract consumption overflow")?;
    }
    Ok(c)
}
pub(super) fn validate_phase(o: &Value, p: &Value) -> Result<()> {
    validate_original(o)?;
    let stage = text(&p["stage"])?;
    let mut allowed = vec!["original", "stage", "consumed", "remaining"];
    if stage != "admitted" {
        allowed.push("observations");
    }
    if stage == "clean" {
        allowed.extend(["credit", "cleanup"]);
    }
    schema(p, "route-phase", &allowed)?;
    ensure(
        p["original"] == reference(o)?,
        "contract immutable original bytes",
    )?;
    ensure(
        bytes(p)?.len() as u64 <= number(&o["maximum_control_bytes"])?,
        "contract actual phase cap",
    )?;
    let reserve = number(&o["reserve"])?;
    if stage == "admitted" {
        return ensure(
            number(&p["consumed"])? == 0 && number(&p["remaining"])? == reserve,
            "contract original full reserve",
        );
    }
    ensure(
        matches!(
            (text(&o["kind"])?, stage),
            ("graph", "published" | "clean") | ("retire", "released" | "unlinked" | "clean")
        ),
        "contract kind phase dispatch",
    )?;
    let c = consumption(o, p)?;
    ensure(
        number(&p["consumed"])? == c
            && reserve
                .checked_sub(c)
                .is_some_and(|n| n >= number(&o["cleanup_bound"]).unwrap_or(u64::MAX)),
        "contract independently derived C and cleanup",
    )?;
    if stage == "clean" {
        fields(&p["cleanup"], &["remaining_roles", "directory_barrier"])?;
        let credit = if o["kind"] == "retire" {
            number(&o["recipe"]["source"]["registered_charge"])?
        } else {
            0
        };
        ensure(
            p["cleanup"]["remaining_roles"] == json!([])
                && p["cleanup"]["directory_barrier"] == true
                && number(&p["credit"])? == credit
                && number(&p["remaining"])? == 0,
            "contract barriered cleanup exact credit",
        )
    } else {
        ensure(
            number(&p["remaining"])? == reserve - c,
            "contract exact R minus C",
        )
    }
}
fn phase_core(o: &Value, p: &Value, core: &Value) -> Result<()> {
    if p["stage"] == "admitted" {
        return ensure(
            *core == o["old"]["ledger"],
            "contract prepublication unchanged ledger",
        );
    }
    let base = &o["old"]["ledger"];
    let credit = if p["stage"] == "clean" {
        number(&p["credit"])?
    } else {
        0
    };
    let total = number(&base["total_charge"])?
        .checked_add(number(&p["consumed"])?)
        .and_then(|n| n.checked_sub(credit))
        .ok_or("contract aggregate overflow")?;
    ensure(
        number(&core["total_charge"])? == total
            && core["standing_control"] == base["standing_control"]
            && core["profile"] == base["profile"]
            && core["incarnation"] == base["incarnation"]
            && core["retained_conversion"] == base["retained_conversion"],
        "contract exact selected ledger settlement",
    )?;
    let selected = tips(core)?;
    ensure(
        selected.len() == array(&p["observations"])?.len(),
        "contract settled tips coverage",
    )?;
    let original = tips(base)?;
    for row in array(&p["observations"])? {
        let allocation = text(&row["allocation_id"])?;
        let tip = selected
            .get(allocation)
            .ok_or("contract selected tip missing")?;
        ensure(
            tip["witness"] == row["witness"]
                && tip["extent"] == row["extent"]
                && tip["registered_charge"] == row["registered_charge"]
                && tip["arena"] == original[allocation]["arena"],
            "contract selected observed tip",
        )?;
    }
    Ok(())
}
pub(super) fn validate_progress(
    o: &Value,
    prior: &Value,
    next: &Value,
    old_core: &Value,
    new_core: &Value,
) -> Result<()> {
    validate_phase(o, prior)?;
    validate_phase(o, next)?;
    let from = text(&prior["stage"])?;
    let to = text(&next["stage"])?;
    ensure(
        matches!(
            (text(&o["kind"])?, from, to),
            ("graph", "admitted", "published")
                | ("graph", "published", "clean")
                | ("retire", "admitted", "released")
                | ("retire", "released", "unlinked")
                | ("retire", "unlinked", "clean")
        ),
        "contract monotone finite phase transition",
    )?;
    if from != "admitted" {
        ensure(
            prior["consumed"] == next["consumed"] && prior["observations"] == next["observations"],
            "contract original one-time publication charge",
        )?;
    }
    phase_core(o, prior, old_core)?;
    phase_core(o, next, new_core)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r() -> Value {
        json!({"kind":"json","sha256":"a".repeat(64),"byte_length":"1"})
    }
    fn witness_for(n: u64) -> Value {
        json!({"profile":PROFILE,"incarnation":INCARNATION,"allocation_id":format!("71000000-0000-4000-8000-{n:012}"),"device":"17","inode":n.to_string()})
    }
    fn fixture(kind: &str) -> (Value, Value, Value) {
        let tips:Vec<_>=(1..=2).map(|n|json!({"witness":witness_for(n),"arena":if n==1{"data"}else{"metadata"},"extent":"4096","registered_charge":"4096"})).collect();
        let ledger = json!({"profile":PROFILE,"incarnation":INCARNATION,"tips":tips,"standing_control":"262144","total_charge":"274432","retained_conversion":null});
        let states =
            json!({"active":{"example.state":"active"},"recovery":{"example.state":"recovery"}});
        let commit = json!({"commit_id":"72000000-0000-4000-8000-000000000001","root_set":{"active":reference(&states["active"]).unwrap(),"recovery":reference(&states["recovery"]).unwrap(),"placement":{"accounting":reference(&ledger).unwrap()},"inventory":r()}});
        let physical = json!({"allocation_id":witness_for(1)["allocation_id"],"arena":"data","offset":"0","byte_length":"17","record_sha256":"b".repeat(64)});
        let source = if kind == "retire" {
            json!({"allocation_id":witness_for(3)["allocation_id"],"witness":witness_for(3),"byte_length":"3000","sha256":"c".repeat(64),"registered_charge":"4096","charge":{},"charge_path":[{}]})
        } else {
            Value::Null
        };
        let o = json!({"schema":{"id":"example.ps2.route-original","version":1},"project_id":"10000000-0000-4000-8000-000000000001","extensions":{},"token":"101","kind":kind,"scope":{"profile":PROFILE,"incarnation":INCARNATION,"codec":"example.ps2.route-v1"},"old":{"head":{"commit_id":commit["commit_id"],"commit_sha256":hash(&commit).unwrap()},"commit":commit,"ledger":ledger,"envelope":null,"overlay":null,"states":states},"target":{"active":r(),"recovery":r(),"operation_index":r(),"conversion_source":null,"base_inventory":r(),"placement":{"generation":"2","active_locator":physical,"recovery_locator":physical,"active_ownership":physical,"recovery_ownership":physical}},"recipe":{"codec":"example.ps2.closed-layout-v1","allocations":[{"allocation_id":witness_for(1)["allocation_id"],"arena":"data","witness":witness_for(1),"old_end":"4096","old_sha256":"d".repeat(64),"end":"8192","sha256":"e".repeat(64),"append_sha256":"f".repeat(64),"frame_count":"1"}],"source":source},"reserve":"32768","project_limit":"2621440","cleanup_bound":"4096","maximum_control_bytes":"32768","maximum_control_count":"8","operation":if kind=="graph"{json!({"intent":r(),"receipt":r()})}else{Value::Null}});
        let admitted = json!({"schema":{"id":"example.ps2.route-phase","version":1},"project_id":o["project_id"],"extensions":{},"original":reference(&o).unwrap(),"stage":"admitted","consumed":"0","remaining":"32768"});
        let mut published = admitted.clone();
        published["stage"] = json!(if kind == "graph" {
            "published"
        } else {
            "released"
        });
        published["consumed"] = json!("4096");
        published["remaining"] = json!("28672");
        published["observations"] = json!([{"allocation_id":witness_for(1)["allocation_id"],"witness":witness_for(1),"extent":"8192","registered_charge":"8192"},{"allocation_id":witness_for(2)["allocation_id"],"witness":witness_for(2),"extent":"4096","registered_charge":"4096"}]);
        (o, admitted, published)
    }
    fn settled(o: &Value, p: &Value) -> Value {
        let mut core = o["old"]["ledger"].clone();
        for tip in core["tips"].as_array_mut().unwrap() {
            let row = p["observations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["witness"] == tip["witness"])
                .unwrap();
            tip["extent"] = row["extent"].clone();
            tip["registered_charge"] = row["registered_charge"].clone();
        }
        let credit = p["credit"].as_str().unwrap_or("0").parse::<u64>().unwrap();
        core["total_charge"] = json!(
            (number(&o["old"]["ledger"]["total_charge"]).unwrap()
                + number(&p["consumed"]).unwrap()
                - credit)
                .to_string()
        );
        core
    }
    #[test]
    fn route_contract_original_phase_and_exact_once_settlement() -> Result<()> {
        for kind in ["graph", "retire"] {
            let (o, a, p) = fixture(kind);
            let core = settled(&o, &p);
            validate_progress(&o, &a, &p, &o["old"]["ledger"], &core)?;
            let mut prior = p;
            if kind == "retire" {
                let mut next = prior.clone();
                next["stage"] = json!("unlinked");
                validate_progress(&o, &prior, &next, &core, &core)?;
                prior = next;
            }
            let mut clean = prior.clone();
            clean["stage"] = json!("clean");
            clean["remaining"] = json!("0");
            clean["credit"] = json!(if kind == "retire" { "4096" } else { "0" });
            clean["cleanup"] = json!({"remaining_roles":[],"directory_barrier":true});
            validate_progress(&o, &prior, &clean, &core, &settled(&o, &clean))?;
            ensure(
                validate_progress(
                    &o,
                    &clean,
                    &clean,
                    &settled(&o, &clean),
                    &settled(&o, &clean),
                )
                .is_err(),
                "repeat transition must use caller terminal retry path",
            )?;
        }
        Ok(())
    }
    #[test]
    fn route_contract_undercharge_witness_extent_and_phase_forgery_refuse() -> Result<()> {
        let (o, a, p) = fixture("graph");
        for mode in 0..6 {
            let mut bad = p.clone();
            match mode {
                0 => bad["consumed"] = json!("0"),
                1 => bad["remaining"] = json!("32768"),
                2 => bad["observations"][0]["witness"]["inode"] = json!("999"),
                3 => bad["observations"][0]["extent"] = json!("8193"),
                4 => bad["observations"][1]["registered_charge"] = json!("0"),
                _ => {
                    let duplicate = bad["observations"][0].clone();
                    bad["observations"].as_array_mut().unwrap().push(duplicate);
                }
            }
            ensure(validate_phase(&o, &bad).is_err(), "phase forgery accepted")?;
        }
        let mut bad = settled(&o, &p);
        bad["total_charge"] = json!("999999");
        ensure(
            validate_progress(&o, &a, &p, &o["old"]["ledger"], &bad).is_err(),
            "aggregate mismatch",
        )?;
        let mut changed = o.clone();
        changed["extensions"] = json!({"example.change":true});
        ensure(
            validate_phase(&changed, &p).is_err(),
            "original byte identity replaced",
        )?;
        Ok(())
    }
    #[test]
    fn route_contract_control_bound_cleanup_and_retirement_skip_refuse() -> Result<()> {
        let (o, a, p) = fixture("retire");
        let mut capacity = o.clone();
        capacity["project_limit"] = capacity["old"]["ledger"]["total_charge"].clone();
        ensure(
            validate_original(&capacity).is_err(),
            "whole original reserve must fit fixed limit",
        )?;
        let mut capacity_overflow = o.clone();
        capacity_overflow["old"]["ledger"]["total_charge"] = json!(u64::MAX.to_string());
        ensure(
            validate_original(&capacity_overflow).is_err(),
            "capacity addition overflow",
        )?;
        let mut under = o.clone();
        under["reserve"] = under["cleanup_bound"].clone();
        ensure(
            validate_original(&under).is_err(),
            "preacceptance missing planned growth",
        )?;
        let mut overflow = o.clone();
        overflow["recipe"]["allocations"][0]["end"] = json!(u64::MAX.to_string());
        ensure(
            validate_original(&overflow).is_err(),
            "preacceptance extent overflow",
        )?;
        let mut malformed = o.clone();
        malformed["extensions"] = json!({"a..b":true});
        ensure(
            validate_original(&malformed).is_err(),
            "malformed qualified extension",
        )?;
        let mut proof = o.clone();
        proof["recipe"]["source"]["charge_path"] = json!([]);
        ensure(
            validate_original(&proof).is_err(),
            "missing source charge reconstruction",
        )?;
        proof["recipe"]["source"]["charge_path"] = json!(vec![json!({}); 257]);
        ensure(
            validate_original(&proof).is_err(),
            "unbounded source charge reconstruction",
        )?;
        let mut wide = o.clone();
        wide["extensions"] = json!({"example.large":"x".repeat(32768)});
        ensure(validate_original(&wide).is_err(), "original body cap")?;
        let mut phase = p.clone();
        phase["extensions"] = json!({"example.large":"x".repeat(32768)});
        ensure(validate_phase(&o, &phase).is_err(), "phase body cap")?;
        let mut clean = p.clone();
        clean["stage"] = json!("clean");
        clean["remaining"] = json!("0");
        clean["credit"] = json!("4096");
        clean["cleanup"] = json!({"remaining_roles":[],"directory_barrier":true});
        let core = settled(&o, &p);
        ensure(
            validate_progress(&o, &a, &clean, &o["old"]["ledger"], &settled(&o, &clean)).is_err(),
            "skip release/unlink",
        )?;
        ensure(
            validate_progress(&o, &p, &clean, &core, &settled(&o, &clean)).is_err(),
            "skip unlink",
        )?;
        clean["cleanup"]["directory_barrier"] = json!(false);
        ensure(
            validate_phase(&o, &clean).is_err(),
            "cleanup before barrier",
        )?;
        Ok(())
    }
    #[test]
    fn route_contract_embedded_old_controls_are_exact_reconstruction() -> Result<()> {
        let (mut o, _, _) = fixture("graph");
        let envelope = json!({"ledger":reference(&o["old"]["ledger"])?});
        let overlay = json!({"base":r(),"controls":[]});
        o["old"]["envelope"] = envelope.clone();
        o["old"]["overlay"] = overlay.clone();
        o["old"]["commit"]["root_set"]["placement"]["accounting"] = reference(&envelope)?;
        o["old"]["commit"]["root_set"]["inventory"] = reference(&overlay)?;
        o["old"]["head"]["commit_sha256"] = json!(hash(&o["old"]["commit"])?);
        validate_original(&o)?;
        for field in ["ledger", "envelope", "overlay", "states"] {
            let mut changed = o.clone();
            changed["old"][field]["example.changed"] = json!(true);
            ensure(
                validate_original(&changed).is_err(),
                "substituted old control accepted",
            )?;
        }
        let mut missing = o.clone();
        missing["old"]["overlay"] = Value::Null;
        ensure(
            validate_original(&missing).is_err(),
            "hidden historical overlay fallback",
        )
    }
}
