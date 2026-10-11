//! Bounded canonical decoding memo, never semantic/physical authority.
use super::{restart::value_storage, wire};
use crate::package::{JsonLimits, parse_canonical_json};
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
#[derive(Default)]
pub(super) struct CanonicalMemo {
    bounds: Option<[usize; 4]>,
    entries: BTreeMap<wire::Key, (Value, usize)>,
    order: VecDeque<wire::Key>,
    pub(super) bytes: usize,
    maximum: Option<usize>,
}
impl CanonicalMemo {
    pub(super) fn set_maximum(&mut self, maximum: usize) {
        self.maximum = Some(maximum);
        self.trim(maximum);
    }
    fn trim(&mut self, maximum: usize) {
        while self.bytes > maximum {
            let Some(key) = self.order.pop_front() else {
                break;
            };
            if let Some((_, cost)) = self.entries.remove(&key) {
                self.bytes -= cost;
            }
        }
    }
    pub(super) fn parse(
        &mut self,
        raw: &[u8],
        key: &wire::Key,
        limits: JsonLimits,
        maximum: usize,
    ) -> wire::Result<(Value, bool)> {
        let maximum = maximum.min(self.maximum.unwrap_or(maximum));
        let bounds = [
            limits.max_bytes,
            limits.max_depth,
            limits.max_members,
            limits.max_array_elements,
        ];
        if self.bounds != Some(bounds) {
            self.entries.clear();
            self.order.clear();
            self.bytes = 0;
            self.bounds = Some(bounds);
        }
        self.trim(maximum);
        // The caller just hashes the actual immutable frame body. No offset,
        // witness, schema or previous Saved token substitutes for these bytes.
        if let Some((value, _)) = self.entries.get(key) {
            return Ok((value.clone(), false));
        }
        let value =
            parse_canonical_json(raw, limits).map_err(|_| "original canonical record budget")?;
        if let Some(cost) = value_storage(&value)
            .and_then(|n| n.checked_add(key.0.capacity()))
            .and_then(|n| n.checked_add(256))
            .filter(|n| *n <= maximum)
        {
            self.trim(maximum - cost);
            self.entries.insert(key.clone(), (value.clone(), cost));
            self.order.push_back(key.clone());
            self.bytes += cost;
        }
        Ok((value, true))
    }
}
