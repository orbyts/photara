//! Private new-attempt input selection. Persisted Originals never enter here.
use super::shared::Bytes;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Measure a fully compiled candidate, then compile its smaller input once.
/// The compiler remains the authority on every bound and byte. Extents occur
/// inside ownership records, so trimming the first candidate is never valid.
/// A changed hash/tree shape may need more room: retain the validated first
/// candidate on that miss instead of changing a codec or relaxing a bound.
pub(super) fn compile<T, E>(
    mut raw: Value,
    mut build: impl FnMut(&Value) -> Result<T, E>,
    files: impl Fn(&T) -> &BTreeMap<String, Bytes>,
) -> Result<(T, Value), E> {
    let mut plan = build(&raw)?;
    if let Some(smaller) = smaller(&raw, files(&plan))
        && let Ok(candidate) = build(&smaller)
    {
        plan = candidate;
        raw = smaller;
    }
    Ok((plan, raw))
}

fn smaller(raw: &Value, files: &BTreeMap<String, Bytes>) -> Option<Value> {
    let mut candidate = raw.clone();
    let mut changed = false;
    for corridor in candidate["planner"]["corridors"].as_array_mut()? {
        let start = corridor["original_end"].as_str()?.parse::<usize>().ok()?;
        let end = corridor["maximum_end"].as_str()?.parse::<usize>().ok()?;
        let bytes = files.get(corridor["allocation_id"].as_str()?)?;
        if bytes.len() != end {
            return None;
        }
        let required = required_end(bytes, start)?;
        // Page-sized slack bounds small changes in ownership refs/tree splits.
        // This is a private sizing quantum, not a persisted geometry or limit.
        let rounded = required.checked_add(4095)? / 4096 * 4096;
        let selected = rounded.min(end);
        if selected < required || selected < start {
            return None;
        }
        changed |= selected < end;
        corridor["maximum_end"] = json!(selected.to_string());
    }
    changed.then_some(candidate)
}

fn required_end(bytes: &[u8], mut at: usize) -> Option<usize> {
    while at < bytes.len() {
        let header_end = at.checked_add(16)?;
        let header = bytes.get(at..header_end)?;
        if &header[..8] != b"PS2PKD01" || header[8] > 3 || header[9..12] != [0; 3] {
            return None;
        }
        let end =
            header_end.checked_add(u32::from_le_bytes(header[12..16].try_into().ok()?) as usize)?;
        if end > bytes.len() {
            return None;
        }
        if header[8] == 0 {
            return (end == bytes.len() && bytes[header_end..].iter().all(|b| *b == 0))
                .then_some(header_end);
        }
        at = end;
    }
    None
}

#[cfg(test)]
mod session_tests {
    use super::*;

    #[test]
    fn sizing_frames_and_failed_refinement_keep_original() {
        let mut bytes = b"PS2PKD01\x01\0\0\0\x03\0\0\0abc".to_vec();
        bytes.extend_from_slice(b"PS2PKD01\0\0\0\0");
        bytes.extend_from_slice(&(16_384_u32 - 35).to_le_bytes());
        bytes.resize(16_384, 0);
        assert_eq!(required_end(&bytes, 0), Some(35));
        let raw = json!({"token":"same-original","planner":{"corridors":[{"allocation_id":"a","original_end":"0","maximum_end":"16384"}]}});
        let files = BTreeMap::from([("a".to_owned(), Bytes::from(bytes.clone()))]);
        let mut calls = 0;
        let (retained, retained_raw) = compile(
            raw.clone(),
            |candidate| {
                calls += 1;
                if calls == 2 {
                    assert_eq!(candidate["token"], raw["token"]);
                    assert_eq!(candidate["planner"]["corridors"][0]["maximum_end"], "4096");
                    Err("changed tree does not fit")
                } else {
                    Ok(files.clone())
                }
            },
            |files| files,
        )
        .unwrap();
        assert_eq!(calls, 2);
        assert_eq!(retained, files);
        assert_eq!(retained_raw, raw);
        bytes[35] = 1;
        assert_eq!(required_end(&bytes, 0), None);
        bytes[35] = 0;
        bytes.truncate(34);
        assert_eq!(required_end(&bytes, 0), None);
        assert!(
            compile(
                raw,
                |_| Err::<BTreeMap<String, Bytes>, _>("original bound"),
                |p| p
            )
            .is_err()
        );
    }
}
