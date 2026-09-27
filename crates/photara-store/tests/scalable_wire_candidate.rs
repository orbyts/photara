//! Unfrozen pure-test PS2 packed byte candidate. No production reader/writer.
//! Semantic leaves are explicit closure probes, not full Core authored validation.
#[path = "scalable_wire_candidate/wire.rs"]
mod wire;
use serde_json::Value;
use wire::{Result, Store, ensure, hash, unhex, verify_package, verify_role};

fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/architecture/proposals/ps2/scalable/linked-vectors.json"
    ))
    .expect("independent fixed vectors")
}

#[test]
fn fixed_canonical_bytes_frames_lengths_and_hashes_match_independent_vectors() -> Result<()> {
    let corpus = corpus();
    ensure(
        corpus["status"] == "unfrozen-pure-closure-candidate",
        "candidate status",
    )?;
    for (digest, entry) in corpus["records"].as_object().ok_or("record pool")? {
        let expected = entry["canonical"].as_str().ok_or("fixed bytes")?.as_bytes();
        ensure(
            photara_core::canonical_json(&entry["input"]).map_err(|_| "serialize")? == expected,
            "fixed canonical bytes",
        )?;
        ensure(
            expected.len() as u64 == entry["byte_length"].as_u64().ok_or("fixed length")?
                && hash(expected) == *digest
                && entry["sha256"] == *digest,
            "fixed canonical digest/length",
        )?;
        if let Some(hex) = entry["frame_hex"].as_str() {
            let frame = unhex(hex)?;
            ensure(
                &frame[..8] == b"PS2PKD01"
                    && u64::from(frame[8]) == entry["frame_tag"].as_u64().ok_or("frame tag")?
                    && frame[9..12] == [0, 0, 0],
                "fixed frame header",
            )?;
            ensure(
                u32::from_le_bytes(frame[12..16].try_into().map_err(|_| "frame length")?) as usize
                    == expected.len()
                    && &frame[16..] == expected
                    && frame.len() as u64
                        == entry["frame_length"].as_u64().ok_or("frame length")?
                    && entry["frame_sha256"] == hash(&frame),
                "fixed exact frame bytes",
            )?;
        }
    }
    Ok(())
}

#[test]
fn complete_bootstrap_and_independent_physical_closures_are_linked() -> Result<()> {
    let corpus = corpus();
    let store = Store::load(&corpus, "valid")?;
    let (active, recovery) = verify_package(&store, 3)?;
    ensure(
        active.logical != recovery.logical
            && active
                .allocations
                .intersection(&recovery.allocations)
                .count()
                == 1,
        "distinct roots share only sealed allocation",
    )?;
    for (role, own, other) in [
        ("active", &active, &recovery),
        ("recovery", &recovery, &active),
    ] {
        let mut isolated = store.clone();
        for allocation in other.allocations.difference(&own.allocations) {
            isolated.allocations.remove(allocation);
        }
        let proof = verify_role(&isolated, role, 3)?;
        let expected = corpus["scenarios"]["valid"]["expected_closures"][role]
            .as_array()
            .ok_or("expected closure")?
            .iter()
            .map(|v| {
                Ok((
                    v["sha256"].as_str().ok_or("expected digest")?.to_owned(),
                    v["byte_length"]
                        .as_str()
                        .ok_or("expected length")?
                        .parse::<u64>()
                        .map_err(|_| "expected decimal")?,
                ))
            })
            .collect::<Result<std::collections::BTreeSet<_>>>()?;
        ensure(proof.logical == expected, "independent fixed closure keys")?;
        ensure(
            proof.logical == own.logical && proof.allocations == own.allocations,
            "independent root without other records",
        )?;
        ensure(
            verify_package(&isolated, 3).is_err(),
            "missing other root cannot open whole package",
        )?;
    }
    Ok(())
}

#[test]
fn linked_semantic_negatives_reach_specific_validation_boundaries() -> Result<()> {
    let corpus = corpus();
    let expected = [
        ("missing-capability", "required scalable capability"),
        ("flat-scalable-confusion", "typed schema/project"),
        (
            "missing-metadata-ownership",
            "exact physical ownership coverage",
        ),
        ("wrong-membership", "wrong locator membership"),
        ("inconsistent-charge", "selected canonical sealed charge"),
        ("dangling-locator", "missing allocation"),
        ("extension-promoted-to-edge", "dangling logical locator"),
        ("recovery-borrows-active", "logical referenced bytes"),
        ("wrong-bootstrap", "RootSet bootstrap/discriminator"),
        ("reference-to-padding", "padding is not referenceable"),
        (
            "extra-owner-locator",
            "exact typed locator membership closure",
        ),
        ("unknown-required-field", "exact fields"),
        ("noncanonical-offset", "canonical decimal"),
    ];
    for (name, error) in expected {
        let store = Store::load(&corpus, name)?;
        assert_eq!(
            verify_package(&store, 3).err(),
            Some(error),
            "linked vector {name}"
        );
    }
    Ok(())
}

#[test]
fn missing_capability_and_reader_refuse_before_any_pack_lookup() -> Result<()> {
    let corpus = corpus();
    let mut missing = Store::load(&corpus, "missing-capability")?;
    missing.allocations.clear();
    assert_eq!(
        verify_package(&missing, 3).err(),
        Some("required scalable capability")
    );
    let mut old_reader = Store::load(&corpus, "valid")?;
    old_reader.allocations.clear();
    assert_eq!(
        verify_package(&old_reader, 2).err(),
        Some("reader capability floor")
    );
    Ok(())
}

#[test]
fn optional_extension_and_predecessor_commitments_are_not_edges() -> Result<()> {
    let corpus = corpus();
    let store = Store::load(&corpus, "valid")?;
    let (active, recovery) = verify_package(&store, 3)?;
    ensure(
        active
            .logical
            .iter()
            .chain(&recovery.logical)
            .all(|(sha, _)| sha != &"f".repeat(64) && sha != &"e".repeat(64)),
        "opaque extension/provenance exclusion",
    )?;
    ensure(
        verify_package(&Store::load(&corpus, "extension-promoted-to-edge")?, 3).is_err(),
        "same bytes in typed field become edge",
    )
}

#[test]
fn altered_frame_bytes_header_and_missing_locator_page_refuse() -> Result<()> {
    let corpus = corpus();
    let original = Store::load(&corpus, "valid")?;
    let metadata = corpus["scenarios"]["valid"]["allocation_ids"]["recovery-meta"]
        .as_str()
        .ok_or("meta ID")?;
    let data = corpus["scenarios"]["valid"]["allocation_ids"]["active-data"]
        .as_str()
        .ok_or("data ID")?;
    for (allocation, offset) in [(metadata, 0), (metadata, 9), (data, 16)] {
        let mut bad = original.clone();
        bad.allocations.get_mut(allocation).ok_or("allocation")?.1[offset] ^= 1;
        ensure(verify_package(&bad, 3).is_err(), "altered framed bytes")?;
    }
    let mut missing = original;
    missing.allocations.remove(metadata);
    ensure(
        verify_package(&missing, 3).is_err(),
        "missing locator implementation allocation",
    )
}

#[test]
fn duplicate_unknown_and_noncanonical_json_refuse() -> Result<()> {
    let corpus = corpus();
    let original = Store::load(&corpus, "valid")?;
    for suffix in [" ", "\n"] {
        let mut bad = original.clone();
        bad.head.extend_from_slice(suffix.as_bytes());
        assert_eq!(verify_package(&bad, 3).err(), Some("canonical JSON"));
    }
    let mut duplicate = original;
    let text = String::from_utf8(duplicate.head.clone()).map_err(|_| "head UTF8")?;
    duplicate.head = text
        .replacen('{', "{\"project_id\":\"duplicate\",", 1)
        .into_bytes();
    assert_eq!(verify_package(&duplicate, 3).err(), Some("canonical JSON"));
    Ok(())
}

#[test]
fn padding_is_exact_bounded_owned_and_nonreferenceable() -> Result<()> {
    let corpus = corpus();
    let original = Store::load(&corpus, "valid")?;
    let allocation = corpus["scenarios"]["valid"]["allocation_ids"]["recovery-meta"]
        .as_str()
        .ok_or("meta ID")?;
    let (arena, bytes) = original
        .allocations
        .get(allocation)
        .ok_or("meta allocation")?;
    let padding = wire::frame_ranges(bytes, arena)?
        .into_iter()
        .find(|(_, _, tag)| *tag == 0)
        .ok_or("padding frame")?;
    for mutation in 0..3 {
        let mut bad = original.clone();
        let bytes = &mut bad.allocations.get_mut(allocation).ok_or("allocation")?.1;
        match mutation {
            0 => {
                let last = bytes.last_mut().ok_or("padding bytes")?;
                *last = 1;
            }
            1 => {
                bytes.pop();
            }
            _ => {
                bytes[padding.0 + 8] = 4;
            }
        }
        ensure(
            verify_package(&bad, 3).is_err(),
            "malformed padding must refuse",
        )?;
    }
    assert_eq!(
        verify_package(&Store::load(&corpus, "reference-to-padding")?, 3).err(),
        Some("padding is not referenceable")
    );
    Ok(())
}
