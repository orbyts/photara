//! `SavedProof2` is retained evidence, never a lease or a durability capability.
use super::{
    JsonLimits, PackageError, Sha256Hex, Value, check, digest, encode, fields, hash, id, json,
    number, records,
};
use crate::package::v1_3::{
    SelectionIdentity,
    repeatable::{checkpoint, journal},
    snapshot,
};

fn jref(value: &Value) -> Result<Value, PackageError> {
    let bytes = encode(value)?;
    Ok(json!({"kind":"json","sha256":hash(&bytes),"byte_length":bytes.len().to_string()}))
}

fn envelope(
    value: &Value,
    kind: &str,
    header: &Value,
    limits: JsonLimits,
) -> Result<(), PackageError> {
    fields(value, &["value", "record_checksum"])?;
    digest(&value["record_checksum"])?;
    check(value["value"]["kind"] == kind)?;
    journal::validate_snapshot_checkpoint(&value["value"], header, limits)
}

pub(super) fn validate(body: &Value, limits: JsonLimits) -> Result<(), PackageError> {
    fields(
        body,
        &[
            "binding",
            "accepted",
            "registration_sha256",
            "journal_header",
            "local_base",
            "acceptance_base",
            "checkpoint",
        ],
    )?;
    records::binding(&body["binding"])?;
    records::accepted(&body["accepted"])?;
    digest(&body["registration_sha256"])?;
    let header = &body["journal_header"];
    let identity = checkpoint::CheckpointIdentity {
        package: SelectionIdentity {
            project: id(&header["project_id"])?,
            library: id(&header["library_id"])?,
            bootstrap_sha256: Sha256Hex::parse(
                header["bootstrap_sha256"]
                    .as_str()
                    .ok_or(PackageError::Record)?,
            )?,
        },
        device: id(&header["device_id"])?,
        incarnation: id(&header["incarnation_id"])?,
    };
    let base = checkpoint::LocalBase::parse(&encode(&body["local_base"])?, &identity, limits)?;
    base.check_header(header)?;
    snapshot::AcceptanceBase::parse(
        &encode(&body["acceptance_base"])?,
        &identity.package,
        limits,
    )?;
    check(
        base.value()["checkpoint"] == jref(&body["acceptance_base"])?
            && body["binding"]["project_id"] == header["project_id"]
            && body["binding"]["incarnation_id"] == header["incarnation_id"]
            && base.value()["coordinate"]["revision"]
                == body["acceptance_base"]["authored_revision"]
            && base.value()["coordinate"]["authored_digest"]
                == body["acceptance_base"]["authored_sha256"],
    )?;
    match body["checkpoint"]["kind"].as_str() {
        Some("operation") => operation(body, limits),
        Some("base") => base_proof(body, &identity, limits),
        _ => Err(PackageError::UnsupportedVersion),
    }
}

fn operation(body: &Value, limits: JsonLimits) -> Result<(), PackageError> {
    let proof = &body["checkpoint"];
    fields(proof, &["kind", "intent", "receipt"])?;
    envelope(
        &proof["intent"],
        "CheckpointIntent",
        &body["journal_header"],
        limits,
    )?;
    envelope(
        &proof["receipt"],
        "CheckpointReceipt",
        &body["journal_header"],
        limits,
    )?;
    let intent = &proof["intent"]["value"];
    let receipt = &proof["receipt"]["value"];
    check(
        receipt["body"]["intent_record_id"] == intent["record_id"]
            && receipt["body"]["intent_record_checksum"] == proof["intent"]["record_checksum"]
            && receipt["body"]["original_sha256"] == hash(&encode(&intent["body"]["original"])?)
            && receipt["body"]["operation_receipt"] == intent["body"]["operation_receipt"]
            && number(&receipt["sequence"])? > number(&intent["sequence"])?
            && intent["body"]["original"]["original_codec"]
                == "photara.codec.ps2-repeatable-admission-v4"
            && intent["body"]["original"]["payload_recipe"]["checkpoint_base"]
                == body["journal_header"]["base"]
            && body["accepted"]["coordinate"]["revision"]
                == receipt["body"]["operation_receipt"]["after"]["revision"]
            && body["accepted"]["coordinate"]["authored_digest"]
                == receipt["body"]["operation_receipt"]["after"]["digest"]
            && body["accepted"]["accepted_frame"] == intent["body"]["accepted_frame"],
    )?;
    records::selected(
        &receipt["body"]["selected_head"],
        &receipt["body"]["selected_commit"],
        &body["binding"]["project_id"],
    )
}

fn base_proof(
    body: &Value,
    identity: &checkpoint::CheckpointIdentity,
    limits: JsonLimits,
) -> Result<(), PackageError> {
    let proof = &body["checkpoint"];
    fields(
        proof,
        &[
            "kind",
            "receipt",
            "selected_commit",
            "original",
            "old_selection",
        ],
    )?;
    let header = &body["journal_header"];
    envelope(&proof["receipt"], "CheckpointBaseReceipt", header, limits)?;
    let record = &proof["receipt"]["value"];
    let receipt = &record["body"];
    let original = &proof["original"];
    let base = &body["local_base"];
    let old =
        checkpoint::LocalSelection::parse(&encode(&proof["old_selection"])?, identity, limits)?;
    fields(
        original,
        &[
            "schema",
            "project_id",
            "library_id",
            "bootstrap_sha256",
            "extensions",
            "maintenance_id",
            "scope",
            "old",
            "target",
            "journal",
            "attempt",
            "writes",
        ],
    )?;
    fields(&original["old"], &["head", "commit", "selector"])?;
    fields(
        &original["target"],
        &[
            "base",
            "state",
            "recovery",
            "pinned_roots",
            "archive_placement",
        ],
    )?;
    fields(
        &original["journal"],
        &["next_stream_id", "through", "lookup", "archive"],
    )?;
    original_shape(original)?;
    check(
        original["schema"]
            == json!({"id":"photara.storage.checkpoint-turnover-original","version":1})
            && original["extensions"] == json!({})
            && receipt["original"] == jref(original)?
            && original["old"]["selector"] == jref(old.value())?
            && old.value()["pending"].is_null()
            && original["maintenance_id"] == record["record_id"]
            && record["record_id"] == receipt["maintenance_id"]
            && number(&record["sequence"])?
                == number(&header["sequence_base"])?
                    .checked_add(1)
                    .ok_or(PackageError::Limit)?
            && number(&record["session_generation"])?
                == number(&old.value()["generation"])?
                    .checked_add(1)
                    .ok_or(PackageError::Limit)?
            && receipt["journal_base"] == jref(base)?
            && receipt["selected_head"] == base["package_head"]
            && receipt["selected_commit"] == base["package_commit"]
            && receipt["selected_commit"] == jref(&proof["selected_commit"])?
            && original["target"]["base"] == base["checkpoint"]
            && original["target"]["state"] == proof["selected_commit"]["root_set"]["active"]
            && original["journal"]["next_stream_id"] == header["stream_id"]
            && original["journal"]["through"] == base["through"]
            && original["journal"]["lookup"] == base["lookup"]
            && original["journal"]["archive"] == base["archive"]
            && header["previous_segment"]["stream_id"] == old.value()["selected"]["stream_id"]
            && header["previous_segment"]["header_sha256"]
                == old.value()["selected"]["header_sha256"]
            && body["accepted"]["coordinate"] == base["coordinate"]
            && body["accepted"]["accepted_frame"] == base["accepted_frame"],
    )?;
    for key in ["project_id", "library_id", "bootstrap_sha256"] {
        check(original[key] == base[key])?;
    }
    let mutation = if base["mutation"].is_null() {
        Value::Null
    } else {
        json!({"record_id":base["mutation"]["record_id"],"record_checksum":base["mutation"]["checksum"]})
    };
    check(body["accepted"]["mutation"] == mutation)?;
    records::selected(
        &base["package_head"],
        &proof["selected_commit"],
        &body["binding"]["project_id"],
    )
}

fn original_shape(original: &Value) -> Result<(), PackageError> {
    fields(&original["scope"], &["profile", "incarnation", "directory"])?;
    fields(&original["scope"]["directory"], &["device", "inode"])?;
    check(original["scope"]["profile"].as_str().is_some())?;
    id(&original["scope"]["incarnation"])?;
    for key in ["device", "inode"] {
        number(&original["scope"]["directory"][key])?;
    }
    records::selected(
        &original["old"]["head"],
        &original["old"]["commit"],
        &original["project_id"],
    )?;
    let mut attempt = original["attempt"].clone();
    check(attempt["planner"]["codec"] == "photara.codec.ps2-checkpoint-turnover-layout-v1")?;
    let corridors = attempt["planner"]["corridors"]
        .as_array_mut()
        .ok_or(PackageError::Record)?;
    let mut ids = std::collections::BTreeSet::new();
    for row in corridors {
        check(ids.insert(id(&row["allocation_id"])?))?;
        check(
            row.get("original_sha256").is_none()
                && row["base_head_sha256"] == hash(&encode(&original["old"]["head"])?),
        )?;
        let object = row.as_object_mut().ok_or(PackageError::Record)?;
        let digest = object
            .remove("base_head_sha256")
            .ok_or(PackageError::Record)?;
        object.insert("original_sha256".into(), digest);
    }
    serde_json::from_value::<crate::package::v1_3::repeatable::Attempt>(attempt)
        .map_err(|_| PackageError::Record)?;
    let mut prior = None;
    for write in original["writes"].as_array().ok_or(PackageError::Record)? {
        fields(
            write,
            &[
                "allocation_id",
                "arena",
                "witness",
                "original_end",
                "maximum_end",
                "target_end",
                "suffix_sha256",
            ],
        )?;
        let key = id(&write["allocation_id"])?;
        check(prior.is_none_or(|last| last < key) && ids.remove(&key))?;
        prior = Some(key);
        let corridor = original["attempt"]["planner"]["corridors"]
            .as_array()
            .ok_or(PackageError::Record)?
            .iter()
            .find(|row| row["allocation_id"] == write["allocation_id"])
            .ok_or(PackageError::Integrity)?;
        for field in ["arena", "witness", "original_end", "maximum_end"] {
            check(write[field] == corridor[field])?;
        }
        digest(&write["suffix_sha256"])?;
        check(
            number(&write["original_end"])? <= number(&write["target_end"])?
                && number(&write["target_end"])? <= number(&write["maximum_end"])?,
        )?;
    }
    check(ids.is_empty())
}
