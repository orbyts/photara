//! Pending origin authority from the actual selected single-birth O/P/hold route.
//! Only publication awaiting original cleanup is exercised; no release policy is added.
use super::{one_birth, origins, phases, wire};
use serde_json::{Value, json};
use wire::{Result, ensure, key, number, reference};
struct Authority {
    root: Value,
    original: Value,
    intent: Value,
}
impl Authority {
    fn selected(
        w: &wire::World,
        expected_original: &Value,
        expected_phase: &Value,
    ) -> Result<Self> {
        let (original, phase) = phases::selected_original(w)?;
        ensure(
            original == *expected_original && phase == *expected_phase,
            "pending exact selected original/phase",
        )?;
        ensure(
            original["original_codec"] == "photara.codec.ps2-single-birth-pending-admission-v1",
            "pending original codec",
        )?;
        // An accepted receipt alone is insufficient: this exact selected operation still
        // owns its unresolved cleanup hold. Terminal clean never qualifies.
        ensure(
            phase["stage"] == "published"
                && phase["cleanup"].is_null()
                && number(&phase["remaining"])? > 0
                && number(&phase["consumed"])?.checked_add(number(&phase["remaining"])?)
                    == Some(number(&original["reserve"])?),
            "pending selected unresolved phase required",
        )?;
        wire::fields(&original["retention_intent"], &["object", "body"])?;
        let intent = original["retention_intent"]["body"].clone();
        ensure(
            reference(&intent) == original["retention_intent"]["object"]
                && intent["operation_id"] == original["request"]["operation_id"]
                && intent["request_sha256"] == original["request"]["request_sha256"],
            "pending original intent commitment",
        )?;
        Ok(Self {
            root: w.commit["root_set"].clone(),
            original,
            intent,
        })
    }
}
impl origins::PendingAuthority for Authority {
    fn operation_id(&self) -> &Value {
        &self.original["request"]["operation_id"]
    }
    fn intent(&self, context: &wire::EvidenceContext<'_>, evidence: &Value) -> Result<Value> {
        ensure(
            *context.root == self.root
                && evidence["original_admission"] == reference(&self.original)
                && evidence["retention_intent"] == reference(&self.intent)
                && evidence["token"] == self.original["token"]
                && evidence["operation_id"] == self.original["request"]["operation_id"]
                && evidence["request_sha256"] == self.original["request"]["request_sha256"],
            "pending actual original evidence binding",
        )?;
        Ok(self.intent.clone())
    }
}
pub(super) fn verify(w: &wire::World) -> Result<wire::Proof> {
    let inputs = one_birth::pending_inputs(w)?;
    let authority = Authority::selected(w, &inputs.original, &inputs.phase)?;
    wire::verify_selected_operation_with(
        w,
        &inputs.old,
        &inputs.prepared.authored,
        &inputs.selection,
        &origins::WithPending(&authority),
    )
}
fn missing_hold(w: &wire::World) -> Result<wire::World> {
    let mut changed = w.clone();
    let old_env_ref = &w.commit["root_set"]["placement"]["accounting"];
    let mut env = w.loose(old_env_ref)?;
    let old_hold_ref = env["holds"].clone();
    let mut hold = w.loose(&old_hold_ref)?;
    for field in ["count", "reserved", "consumed", "remaining"] {
        hold[field] = json!("0");
    }
    hold["entries"] = json!([]);
    env["holds"] = reference(&hold);
    let old_overlay_ref = &w.commit["root_set"]["inventory"];
    let mut overlay = w.loose(old_overlay_ref)?;
    let controls = overlay["controls"]
        .as_array_mut()
        .ok_or("pending overlay controls")?;
    for r in controls.iter_mut() {
        if *r == old_hold_ref {
            *r = reference(&hold);
        } else if *r == *old_env_ref {
            *r = reference(&env);
        }
    }
    controls.sort_by_key(|r| key(r).unwrap());
    for old in [&old_hold_ref, old_env_ref, old_overlay_ref] {
        changed.loose.remove(&key(old)?.0);
    }
    for value in [&hold, &env, &overlay] {
        changed
            .loose
            .insert(key(&reference(value))?.0, wire::encode(value));
    }
    changed.commit["root_set"]["placement"]["accounting"] = reference(&env);
    changed.commit["root_set"]["inventory"] = reference(&overlay);
    changed.rehash_head();
    Ok(changed)
}
#[test]
fn selected_pending_origin_requires_actual_unresolved_hold_and_exact_intent_subset() {
    let selected = one_birth::selected_pending("published", false).unwrap();
    let proof = verify(&selected).unwrap();
    assert_eq!(proof.roles["active"].receipts.len(), 4);
    assert!(
        proof.roles["active"]
            .associations
            .values()
            .any(|a| a["origin"]["kind"] == "pending")
    );
    let terminal = one_birth::selected_pending("clean", false).unwrap();
    assert_eq!(
        verify(&terminal).err(),
        Some("pending selected unresolved phase required")
    );
    assert_eq!(
        verify(&missing_hold(&selected).unwrap()).err(),
        Some("phase sole original hold")
    );
    let mismatch = one_birth::selected_pending("published", true).unwrap();
    assert_eq!(
        verify(&mismatch).err(),
        Some("origin exact selected subset union")
    );
}
