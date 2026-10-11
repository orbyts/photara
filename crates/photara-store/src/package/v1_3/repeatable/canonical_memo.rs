//! Bounded canonical-validation certificates, never semantic/physical authority.
use super::wire;
use crate::package::{JsonLimits, parse_canonical_json};
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
#[derive(Clone, Debug)]
pub(super) struct Summary {
    pub root: Option<String>,
    pub association: Option<String>,
}
impl Summary {
    pub(super) fn from_value(v: &Value) -> Self {
        Self {
            root: (v["schema"]["id"] == "photara.package.state-root")
                .then(|| v["root_id"].as_str().map(str::to_owned))
                .flatten(),
            association: v["association_id"].as_str().map(str::to_owned),
        }
    }
}
#[derive(Default)]
pub(super) struct CanonicalMemo {
    bounds: Option<[usize; 4]>,
    entries: BTreeMap<wire::Key, (Summary, usize)>,
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
    pub(super) fn validate(
        &mut self,
        raw: &[u8],
        key: &wire::Key,
        limits: JsonLimits,
        maximum: usize,
    ) -> wire::Result<Summary> {
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
        // Caller freshly hashed exact immutable bytes; changed parser limits clear certificates.
        if let Some((summary, _)) = self.entries.get(key) {
            return Ok(summary.clone());
        }
        let value =
            parse_canonical_json(raw, limits).map_err(|_| "original canonical record budget")?;
        super::layout::count_decode();
        let summary = Summary::from_value(&value);
        if let Some(cost) = key
            .0
            .capacity()
            .checked_mul(2)
            .and_then(|n| n.checked_add(512))
            .and_then(|n| n.checked_add(summary.root.as_ref().map_or(0, String::capacity)))
            .and_then(|n| n.checked_add(summary.association.as_ref().map_or(0, String::capacity)))
            .filter(|n| *n <= maximum)
        {
            self.trim(maximum - cost);
            self.entries.insert(key.clone(), (summary.clone(), cost));
            self.order.push_back(key.clone());
            self.bytes += cost;
        }
        Ok(summary)
    }
}
