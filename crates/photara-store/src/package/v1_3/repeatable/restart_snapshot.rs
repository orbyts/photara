//! Exact independent inputs used by one completed journal-prefix proof.
use super::{
    AllocationInspection, AllocationObservation, BTreeMap, PackageError, PackageUuid, RefCell,
    Value, check, value_storage,
};
use crate::package::Sha256Hex;
use serde_json::json;
#[derive(Clone)]
pub(in super::super) struct PrefixSnapshot {
    context: String,
    allocations: BTreeMap<String, (String, Value, usize, String)>,
}
impl PrefixSnapshot {
    pub(in super::super) fn new(package: &super::OriginalPackage, context: String) -> Self {
        Self {
            context,
            allocations: package
                .allocations
                .iter()
                .map(|(id, a)| {
                    (
                        id.clone(),
                        (
                            a.arena.clone(),
                            a.witness.clone(),
                            a.bytes.len(),
                            super::wire::hash(&a.bytes),
                        ),
                    )
                })
                .collect(),
        }
    }
    pub(in super::super) fn matches(
        &self,
        package: &super::OriginalPackage,
        context: &str,
        tips: &std::collections::BTreeSet<PackageUuid>,
        source: &impl AllocationInspection,
        maximum: (u64, u64),
    ) -> Result<bool, PackageError> {
        if self.context != context || self.allocations.len() != package.allocations.len() {
            return Ok(false);
        }
        let mut total = 0u64;
        for (id, (arena, witness, length, hash)) in &self.allocations {
            let Some(actual) = package.allocations.get(id) else {
                return Ok(false);
            };
            let id = PackageUuid::parse(id)?;
            total = total
                .checked_add(actual.bytes.len() as u64)
                .ok_or(PackageError::Limit)?;
            if actual.arena != *arena
                || actual.witness != *witness
                || (!tips.contains(&id) && actual.bytes.len() != *length)
                || actual.bytes.len() < *length
                || actual.bytes.len() as u64 > maximum.0
                || total > maximum.1
                || source.observe(id)?.extent != actual.bytes.len() as u64
                || super::wire::hash(&actual.bytes[..*length]) != *hash
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(in super::super) fn cost(&self) -> Option<usize> {
        value_storage(&json!([self.context, self.allocations]))
    }
}
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
    // Historical completed proofs already used Prefix's monotone tip view.
    // Replaying that proof permits only those same registered tips to append;
    // all immutable observations and every queried old prefix remain exact.
    pub(in super::super) fn extended(
        &self,
        source: &impl AllocationInspection,
        tips: &std::collections::BTreeSet<PackageUuid>,
    ) -> Result<Option<Self>, PackageError> {
        let mut fresh = self.clone();
        for (id, expected) in &self.values {
            let actual = value(&source.observe(*id)?);
            if actual != *expected {
                if !tips.contains(id)
                    || actual[0] != expected[0]
                    || actual[1] != "FramedJson"
                    || actual[1] != expected[1]
                    || actual[4] != expected[4]
                    || actual[5] != expected[5]
                    || actual[2].as_u64().ok_or(PackageError::Record)?
                        < expected[2].as_u64().ok_or(PackageError::Record)?
                    || actual[3].as_u64().ok_or(PackageError::Record)?
                        < expected[3].as_u64().ok_or(PackageError::Record)?
                {
                    return Ok(None);
                }
                fresh.values.insert(*id, actual);
            }
        }
        for ((id, length, max), expected) in &self.prefixes {
            if source.framed_prefix_digest(*id, *length, *max)? != *expected {
                return Ok(None);
            }
        }
        Ok(Some(fresh))
    }
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
