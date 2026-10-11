//! Exact v3 batch overlays. Supplied bytes remain untrusted until every edge audits.
use super::{
    TreeLimits,
    selected::{json_ref, schema},
    tree::{fields, number},
};
use crate::package::{ObjectRef, PackageError, PackageUuid};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, PackageError>;
fn check(ok: bool) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(PackageError::Integrity)
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Family<'a> {
    Inventory {
        purpose: Option<&'a str>,
    },
    Locator {
        role: Option<&'a str>,
        allocation: &'a str,
    },
}
impl Family<'_> {
    fn prefix(self) -> &'static str {
        match self {
            Self::Inventory { .. } => "photara.package.inventory-overlay",
            Self::Locator { .. } => "photara.storage.locator-overlay",
        }
    }
    fn normal(self) -> &'static str {
        match self {
            Self::Inventory { .. } => "photara.package.inventory",
            Self::Locator { .. } => "photara.storage.locator",
        }
    }
    fn object(self, row: &Value) -> Result<ObjectRef> {
        let value = if matches!(self, Self::Inventory { .. }) {
            row
        } else {
            &row["object"]
        };
        if matches!(self, Self::Inventory { .. }) {
            json_ref(value)
        } else {
            super::locator::validate_entry(row)?;
            serde_json::from_value(value.clone()).map_err(|_| PackageError::Record)
        }
    }
}
pub(crate) struct Resolved {
    pub entries: Vec<Value>,
    pub nodes: Vec<(Value, Value)>,
    pub base: Value,
    pub base_count: u64,
    pub index: Option<Value>,
    pub batches: Vec<Value>,
    pub sequence: u64,
}
struct Audit<'a, F> {
    load: F,
    project: PackageUuid,
    family: Family<'a>,
    limits: TreeLimits,
    nodes: Vec<(Value, Value)>,
    seen: BTreeSet<Vec<u8>>,
    work: usize,
    batch_depths: BTreeMap<u64, usize>,
}
impl<F: FnMut(&Value) -> Result<Value>> Audit<'_, F> {
    fn read(&mut self, r: &Value, depth: usize) -> Result<Value> {
        if depth >= self.limits.max_depth || self.nodes.len() >= self.limits.max_pages {
            return Err(PackageError::Limit);
        }
        if let Family::Locator { allocation, .. } = self.family {
            let _: super::PhysicalRef =
                serde_json::from_value(r.clone()).map_err(|_| PackageError::Record)?;
            check(r["allocation_id"] == allocation && r["arena"] == "metadata")?;
        } else {
            json_ref(r)?;
        }
        let key = photara_core::canonical_json(r).map_err(|_| PackageError::Record)?;
        check(self.seen.insert(key))?;
        let value = (self.load)(r)?;
        self.nodes.push((r.clone(), value.clone()));
        Ok(value)
    }
    fn shape(&self, v: &Value, name: &str, keys: &[&str], strict: bool) -> Result<()> {
        schema(
            v,
            name,
            if name == "photara.package.inventory-overlay" {
                2
            } else {
                1
            },
            keys,
            self.project,
        )?;
        if strict {
            check(v["extensions"] == json!({}))?;
        }
        Ok(())
    }
    fn header(&self, header: &Value) -> Result<()> {
        match self.family {
            Family::Inventory { purpose } => {
                self.shape(
                    header,
                    self.family.prefix(),
                    &[
                        "purpose",
                        "base",
                        "base_count",
                        "batches",
                        "through_sequence",
                        "count",
                    ],
                    true,
                )?;
                check(
                    matches!(header["purpose"].as_str(), Some("state" | "global"))
                        && purpose.is_none_or(|p| header["purpose"] == p),
                )?;
            }
            Family::Locator { role, allocation } => {
                self.shape(
                    header,
                    self.family.prefix(),
                    &[
                        "role",
                        "allocation_id",
                        "base",
                        "base_count",
                        "batches",
                        "through_sequence",
                        "count",
                    ],
                    true,
                )?;
                check(
                    header["allocation_id"] == allocation
                        && header["role"].as_str().is_some_and(|r| !r.is_empty())
                        && role.is_none_or(|r| header["role"] == r),
                )?;
            }
        }
        Ok(())
    }
    fn charge(&mut self, n: usize) -> Result<()> {
        self.work = self.work.checked_add(n).ok_or(PackageError::Limit)?;
        if self.work > self.limits.max_entries {
            return Err(PackageError::Limit);
        }
        Ok(())
    }
    fn tree(&mut self, r: &Value, index: bool, depth: usize) -> Result<Vec<Value>> {
        let v = self.read(r, depth)?;
        let prefix = if index {
            format!("{}-index", self.family.prefix())
        } else {
            self.family.normal().into()
        };
        let leaf = v["schema"]["id"] == format!("{prefix}-leaf");
        self.shape(
            &v,
            &format!("{prefix}-{}", if leaf { "leaf" } else { "branch" }),
            &["count", if leaf { "entries" } else { "children" }],
            index,
        )?;
        let mut result = Vec::new();
        if leaf {
            let rows = v["entries"].as_array().ok_or(PackageError::Record)?;
            if rows.len() > self.limits.max_leaf_entries {
                return Err(PackageError::Limit);
            }
            self.charge(rows.len())?;
            for row in rows {
                if index {
                    fields(row, &["sequence", "batch"])?;
                    let sequence = number(&row["sequence"])?;
                    check(self.batch_depths.insert(sequence, depth + 1).is_none())?;
                } else {
                    self.family.object(row)?;
                }
            }
            result.clone_from(rows);
        } else {
            let rows = v["children"].as_array().ok_or(PackageError::Record)?;
            check(rows.len() >= 2)?;
            if rows.len() > self.limits.max_branch_children {
                return Err(PackageError::Limit);
            }
            for row in rows {
                fields(row, &["first", "last", "count", "child"])?;
                let child = self.tree(&row["child"], index, depth + 1)?;
                check(!child.is_empty() && number(&row["count"])? == child.len() as u64)?;
                let bound = |v: &Value| {
                    if index {
                        v["sequence"].clone()
                    } else if matches!(self.family, Family::Inventory { .. }) {
                        v.clone()
                    } else {
                        v["object"].clone()
                    }
                };
                check(
                    row["first"] == bound(&child[0])
                        && row["last"] == bound(child.last().ok_or(PackageError::Integrity)?),
                )?;
                result.extend(child);
            }
        }
        check(number(&v["count"])? == result.len() as u64)?;
        for pair in result.windows(2) {
            check(if index {
                number(&pair[0]["sequence"])? < number(&pair[1]["sequence"])?
            } else {
                self.family.object(&pair[0])? < self.family.object(&pair[1])?
            })?;
        }
        Ok(result)
    }
}
#[expect(
    clippy::too_many_lines,
    reason = "One exact overlay audit follows anchor, index, and every ordered transition"
)]
pub(crate) fn resolve(
    mut load: impl FnMut(&Value) -> Result<Value>,
    root: &Value,
    project: PackageUuid,
    family: Family<'_>,
    limits: TreeLimits,
) -> Result<Resolved> {
    let first = load(root)?;
    let is_overlay = first["schema"]["id"] == family.prefix()
        && first["schema"]["version"]
            == if matches!(family, Family::Inventory { .. }) {
                2
            } else {
                1
            };
    let mut audit = Audit {
        load,
        project,
        family,
        limits,
        nodes: Vec::new(),
        seen: BTreeSet::new(),
        work: 0,
        batch_depths: BTreeMap::new(),
    };
    if !is_overlay {
        let entries = audit.tree(root, false, 0)?;
        return Ok(Resolved {
            base_count: entries.len() as u64,
            entries,
            nodes: audit.nodes,
            base: root.clone(),
            index: None,
            batches: Vec::new(),
            sequence: 0,
        });
    }
    let header = audit.read(root, 0)?;
    audit.header(&header)?;
    let anchor = audit.tree(&header["base"], false, 1)?;
    let base_count = number(&header["base_count"])?;
    check(base_count == anchor.len() as u64)?;
    let mut current = anchor
        .into_iter()
        .map(|v| Ok((family.object(&v)?, v)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let batches = audit.tree(&header["batches"], true, 1)?;
    let sequence = number(&header["through_sequence"])?;
    check(sequence > 0 && sequence == batches.len() as u64)?;
    for (index, row) in batches.iter().enumerate() {
        check(number(&row["sequence"])? == index as u64 + 1)?;
        let depth = *audit
            .batch_depths
            .get(&(index as u64 + 1))
            .ok_or(PackageError::Integrity)?;
        let batch = audit.read(&row["batch"], depth)?;
        audit.shape(
            &batch,
            &format!("{}-batch", family.prefix()),
            &["sequence", "before_count", "after_count", "entries"],
            true,
        )?;
        check(
            batch["sequence"] == row["sequence"]
                && number(&batch["before_count"])? == current.len() as u64,
        )?;
        let entries = batch["entries"].as_array().ok_or(PackageError::Record)?;
        check(!entries.is_empty())?;
        audit.charge(entries.len())?;
        let mut previous = None;
        for entry in entries {
            fields(entry, &["object", "change"])?;
            let object: ObjectRef = serde_json::from_value(entry["object"].clone())
                .map_err(|_| PackageError::Record)?;
            if matches!(family, Family::Inventory { .. }) {
                json_ref(&entry["object"])?;
            }
            let order = (object.sha256.clone(), object.byte_length.get());
            check(previous.as_ref().is_none_or(|p| p < &order))?;
            previous = Some(order);
            match family {
                Family::Inventory { .. } => match entry["change"].as_str() {
                    Some("insert") => {
                        check(current.insert(object, entry["object"].clone()).is_none())?;
                    }
                    Some("remove") => check(current.remove(&object).is_some())?,
                    _ => return Err(PackageError::Record),
                },
                Family::Locator { .. } => match entry["change"]["kind"].as_str() {
                    Some("put") => {
                        fields(&entry["change"], &["kind", "membership", "physical"])?;
                        check(matches!(
                            entry["change"]["membership"].as_str(),
                            Some("semantic" | "ownership" | "blob")
                        ))?;
                        let value = json!({"object":entry["object"],"membership":entry["change"]["membership"],"physical":entry["change"]["physical"]});
                        super::locator::validate_entry(&value)?;
                        check(current.insert(object, value.clone()).as_ref() != Some(&value))?;
                    }
                    Some("remove") => {
                        fields(&entry["change"], &["kind"])?;
                        check(current.remove(&object).is_some())?;
                    }
                    _ => return Err(PackageError::Record),
                },
            }
            if current.len() > limits.max_entries {
                return Err(PackageError::Limit);
            }
        }
        let keys = current
            .keys()
            .map(|r| (r.sha256.clone(), r.byte_length.get()))
            .collect::<BTreeSet<_>>();
        check(keys.len() == current.len())?;
        check(number(&batch["after_count"])? == current.len() as u64)?;
    }
    check(number(&header["count"])? == current.len() as u64)?;
    Ok(Resolved {
        entries: current.into_values().collect(),
        nodes: audit.nodes,
        base: header["base"].clone(),
        base_count,
        index: Some(header["batches"].clone()),
        batches,
        sequence,
    })
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod tests;
#[cfg(test)]
pub(crate) use tests::audit_regressions;

#[path = "overlay_scoped.rs"]
mod scoped;
pub(crate) use scoped::{lookup, put_basis};
