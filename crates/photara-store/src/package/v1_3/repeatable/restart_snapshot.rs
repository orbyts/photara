//! Exact independent inputs used by one completed journal-prefix proof.
use super::{
    AllocationInspection, AllocationObservation, BTreeMap, PackageError, PackageUuid, RefCell,
    Value, check, value_storage,
};
use crate::package::Sha256Hex;
use serde_json::json;
#[derive(Clone, Default)]
pub(in super::super) struct Observations {
    values: BTreeMap<PackageUuid, Value>,
    prefixes: BTreeMap<(PackageUuid, u64, u64), Sha256Hex>,
}
fn value(v: &AllocationObservation) -> Value {
    json!([
        v.arena,
        format!("{:?}", v.layout),
        v.extent,
        v.registered_charge,
        v.device,
        v.inode
    ])
}
impl Observations {
    pub(in super::super) fn matches(
        &self,
        source: &impl AllocationInspection,
    ) -> Result<bool, PackageError> {
        for (id, expected) in &self.values {
            if value(&source.observe(*id)?) != *expected {
                return Ok(false);
            }
        }
        for ((id, length, max), expected) in &self.prefixes {
            if source.framed_prefix_digest(*id, *length, *max)? != *expected {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(in super::super) fn cost(&self) -> Option<usize> {
        value_storage(
            &json!({"values":self.values,"prefixes":self.prefixes.iter().map(|(k,v)|json!([k.0,k.1,k.2,v])).collect::<Vec<_>>()}),
        )
    }
}
pub(in super::super) struct Recording<'a, I> {
    source: &'a I,
    observed: RefCell<Observations>,
}
impl<'a, I> Recording<'a, I> {
    pub(in super::super) fn new(source: &'a I, earlier: Observations) -> Self {
        Self {
            source,
            observed: RefCell::new(earlier),
        }
    }
    pub(in super::super) fn finish(self) -> Observations {
        self.observed.into_inner()
    }
}
impl<I: AllocationInspection> AllocationInspection for Recording<'_, I> {
    fn observe(&self, id: PackageUuid) -> Result<AllocationObservation, PackageError> {
        let result = self.source.observe(id)?;
        let encoded = value(&result);
        if let Some(prior) = self
            .observed
            .borrow_mut()
            .values
            .insert(id, encoded.clone())
        {
            check(prior == encoded)?;
        }
        Ok(result)
    }
    fn framed_prefix_digest(
        &self,
        id: PackageUuid,
        length: u64,
        max: u64,
    ) -> Result<Sha256Hex, PackageError> {
        let result = self.source.framed_prefix_digest(id, length, max)?;
        if let Some(prior) = self
            .observed
            .borrow_mut()
            .prefixes
            .insert((id, length, max), result.clone())
        {
            check(prior == result)?;
        }
        Ok(result)
    }
}

/// Two adjacent final selectors are the only useful executor working set. This
/// optional bound affects reuse only, never acceptance or stored package limits.
#[derive(Default)]
pub(in super::super) struct Candidates {
    entries: std::collections::VecDeque<(String, Observations, usize)>,
    pub(in super::super) bytes: usize,
}
impl Candidates {
    pub(in super::super) fn trim(&mut self, maximum: usize) {
        while self.bytes > maximum || self.entries.len() > 2 {
            if let Some((_, _, cost)) = self.entries.pop_front() {
                self.bytes -= cost;
            } else {
                break;
            }
        }
    }
    pub(in super::super) fn matches(
        &self,
        key: &str,
        source: &impl AllocationInspection,
    ) -> Result<bool, PackageError> {
        match self.entries.iter().find(|(k, _, _)| k == key) {
            Some((_, values, _)) => values.matches(source),
            None => Ok(false),
        }
    }
    pub(in super::super) fn insert(
        &mut self,
        key: String,
        observations: Observations,
        maximum: usize,
    ) {
        if let Some(at) = self.entries.iter().position(|(k, _, _)| k == &key)
            && let Some((_, _, cost)) = self.entries.remove(at)
        {
            self.bytes -= cost;
        }
        if let Some(cost) = observations
            .cost()
            .and_then(|n| n.checked_add(key.capacity()))
            && cost <= maximum
        {
            self.bytes += cost;
            self.entries.push_back((key, observations, cost));
        }
        self.trim(maximum);
    }
}
