//! Scoped authenticated reads; publication still requires the full overlay audit.
use super::{
    Audit, BTreeMap, BTreeSet, Family, ObjectRef, PackageError, PackageUuid, Result, TreeLimits,
    Value, check, fields, json, json_ref, number,
};

fn validate_reference(family: Family<'_>, reference: &Value) -> Result<()> {
    match family {
        Family::Inventory { .. } => {
            json_ref(reference)?;
        }
        Family::Locator { allocation, .. } => {
            let _: super::super::PhysicalRef =
                serde_json::from_value(reference.clone()).map_err(|_| PackageError::Record)?;
            check(reference["allocation_id"] == allocation && reference["arena"] == "metadata")?;
        }
    }
    Ok(())
}

impl<F: FnMut(&Value) -> Result<Value>> Audit<'_, F> {
    fn anchor_lookup(
        &mut self,
        root: &Value,
        key: &ObjectRef,
        count: u64,
    ) -> Result<Option<Value>> {
        let mut reference = root.clone();
        let mut expected = (None, None, count);
        for depth in 1..self.limits.max_depth {
            let page = self.read(&reference, depth)?;
            let leaf = page["schema"]["id"] == format!("{}-leaf", self.family.normal());
            self.shape(
                &page,
                &format!(
                    "{}-{}",
                    self.family.normal(),
                    if leaf { "leaf" } else { "branch" }
                ),
                &["count", if leaf { "entries" } else { "children" }],
                false,
            )?;
            check(number(&page["count"])? == expected.2)?;
            if leaf {
                let entries = page["entries"].as_array().ok_or(PackageError::Record)?;
                if entries.len() > self.limits.max_leaf_entries {
                    return Err(PackageError::Limit);
                }
                self.charge(entries.len())?;
                check(entries.len() as u64 == expected.2)?;
                let keys = entries
                    .iter()
                    .map(|v| self.family.object(v))
                    .collect::<Result<Vec<_>>>()?;
                check(keys.windows(2).all(|p| p[0] < p[1]))?;
                if let Some(first) = expected.0 {
                    check(keys.first() == Some(&first) && keys.last() == expected.1.as_ref())?;
                }
                return Ok(keys
                    .iter()
                    .position(|k| k == key)
                    .map(|i| entries[i].clone()));
            }
            let children = page["children"].as_array().ok_or(PackageError::Record)?;
            check(children.len() >= 2)?;
            if children.len() > self.limits.max_branch_children {
                return Err(PackageError::Limit);
            }
            let mut total = 0u64;
            let mut previous = None;
            let mut first_bound = None;
            let mut chosen = None;
            for child in children {
                fields(child, &["first", "last", "count", "child"])?;
                let first: ObjectRef = serde_json::from_value(child["first"].clone())
                    .map_err(|_| PackageError::Record)?;
                let last: ObjectRef = serde_json::from_value(child["last"].clone())
                    .map_err(|_| PackageError::Record)?;
                for bound in [&first, &last] {
                    check(
                        bound.kind != crate::package::ObjectKind::Json
                            || bound.byte_length.get() > 0,
                    )?;
                    if matches!(self.family, Family::Inventory { .. }) {
                        check(bound.kind == crate::package::ObjectKind::Json)?;
                    }
                }
                validate_reference(self.family, &child["child"])?;
                let count = number(&child["count"])?;
                check(count > 0 && first <= last && previous.as_ref().is_none_or(|p| p < &first))?;
                total = total.checked_add(count).ok_or(PackageError::Limit)?;
                first_bound.get_or_insert_with(|| first.clone());
                previous = Some(last.clone());
                if &first <= key && key <= &last {
                    chosen = Some((child["child"].clone(), (Some(first), Some(last), count)));
                }
            }
            check(total == expected.2)?;
            if expected.0.is_some() {
                check(expected.0 == first_bound && expected.1 == previous)?;
            }
            let Some((child, summary)) = chosen else {
                return Ok(None);
            };
            reference = child;
            expected = summary;
        }
        Err(PackageError::Limit)
    }
}

/// Checks the selected key only. Unaccessed anchor pages and superseded batches
/// have no integrity claim; only `resolve` can certify the complete history.
pub(crate) fn lookup(
    load: impl FnMut(&Value) -> Result<Value>,
    root: &Value,
    project: PackageUuid,
    family: Family<'_>,
    limits: TreeLimits,
    key: &ObjectRef,
) -> Result<Option<Value>> {
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
    let header = audit.read(root, 0)?;
    audit.header(&header)?;
    validate_reference(family, &header["base"])?;
    let base_count = number(&header["base_count"])?;
    let mut after = number(&header["count"])?;
    if base_count > limits.max_entries as u64 || after > limits.max_entries as u64 {
        return Err(PackageError::Limit);
    }
    let batches = audit.tree(&header["batches"], true, 1)?;
    let sequence = number(&header["through_sequence"])?;
    check(sequence > 0 && sequence == batches.len() as u64)?;
    for (index, row) in batches.iter().enumerate() {
        check(number(&row["sequence"])? == index as u64 + 1)?;
    }
    for row in batches.iter().rev() {
        let sequence = number(&row["sequence"])?;
        let depth = *audit
            .batch_depths
            .get(&sequence)
            .ok_or(PackageError::Integrity)?;
        let batch = audit.read(&row["batch"], depth)?;
        audit.shape(
            &batch,
            &format!("{}-batch", family.prefix()),
            &["sequence", "before_count", "after_count", "entries"],
            true,
        )?;
        check(batch["sequence"] == row["sequence"] && number(&batch["after_count"])? == after)?;
        after = number(&batch["before_count"])?;
        if after > limits.max_entries as u64 {
            return Err(PackageError::Limit);
        }
        let entries = batch["entries"].as_array().ok_or(PackageError::Record)?;
        check(!entries.is_empty())?;
        audit.charge(entries.len())?;
        let mut previous = None;
        let mut found = None;
        for entry in entries {
            fields(entry, &["object", "change"])?;
            let object: ObjectRef = serde_json::from_value(entry["object"].clone())
                .map_err(|_| PackageError::Record)?;
            let order = (object.sha256.clone(), object.byte_length.get());
            check(previous.as_ref().is_none_or(|p| p < &order))?;
            previous = Some(order);
            let value = match family {
                Family::Inventory { .. } => {
                    json_ref(&entry["object"])?;
                    match entry["change"].as_str() {
                        Some("insert") => Some(entry["object"].clone()),
                        Some("remove") => None,
                        _ => return Err(PackageError::Record),
                    }
                }
                Family::Locator { .. } => match entry["change"]["kind"].as_str() {
                    Some("put") => {
                        fields(&entry["change"], &["kind", "membership", "physical"])?;
                        let value = json!({"object":entry["object"],"membership":entry["change"]["membership"],"physical":entry["change"]["physical"]});
                        super::super::locator::validate_entry(&value)?;
                        Some(value)
                    }
                    Some("remove") => {
                        fields(&entry["change"], &["kind"])?;
                        None
                    }
                    _ => return Err(PackageError::Record),
                },
            };
            if object == *key {
                found = Some(value);
            }
        }
        if let Some(value) = found {
            return Ok(value);
        }
    }
    check(after == base_count)?;
    audit.anchor_lookup(&header["base"], key, base_count)
}

/// Validated metadata and exact scoped changes for an append-only locator update.
/// This does not certify unrequested historical branches or payload records.
pub(crate) struct PutBasis {
    pub base: Value,
    pub base_count: u64,
    pub index: Option<Value>,
    pub batches: Vec<Value>,
    pub sequence: u64,
    pub before: u64,
    pub after: u64,
    pub changes: Vec<Value>,
}
pub(crate) fn put_basis(
    mut load: impl FnMut(&Value) -> Result<Value>,
    root: &Value,
    project: PackageUuid,
    family: Family<'_>,
    limits: TreeLimits,
    target: &[Value],
) -> Result<PutBasis> {
    check(matches!(family, Family::Locator { .. }) && !target.is_empty())?;
    if target.len() > limits.max_entries {
        return Err(PackageError::Limit);
    }
    let header = load(root)?;
    let overlay = header["schema"]["id"] == family.prefix();
    let before = number(&header["count"])?;
    if before > limits.max_entries as u64 {
        return Err(PackageError::Limit);
    }
    let (base, base_count, index, batches, sequence) = if overlay {
        let mut audit = Audit {
            load: &mut load,
            project,
            family,
            limits,
            nodes: Vec::new(),
            seen: BTreeSet::new(),
            work: 0,
            batch_depths: BTreeMap::new(),
        };
        audit.header(&header)?;
        validate_reference(family, &header["base"])?;
        let rows = audit.tree(&header["batches"], true, 1)?;
        let sequence = number(&header["through_sequence"])?;
        check(sequence > 0 && sequence == rows.len() as u64)?;
        for (i, row) in rows.iter().enumerate() {
            check(number(&row["sequence"])? == i as u64 + 1)?;
        }
        let count = number(&header["base_count"])?;
        if count > limits.max_entries as u64 {
            return Err(PackageError::Limit);
        }
        (
            header["base"].clone(),
            count,
            Some(header["batches"].clone()),
            rows,
            sequence,
        )
    } else {
        (root.clone(), before, None, Vec::new(), 0)
    };
    let mut after = before;
    let mut changes = Vec::new();
    let mut previous = None;
    for value in target {
        let object = family.object(value)?;
        let order = (object.sha256.clone(), object.byte_length.get());
        check(previous.as_ref().is_none_or(|p| p < &order))?;
        previous = Some(order);
        let old = if overlay {
            lookup(&mut load, root, project, family, limits, &object)?
        } else {
            let mut audit = Audit {
                load: &mut load,
                project,
                family,
                limits,
                nodes: Vec::new(),
                seen: BTreeSet::new(),
                work: 0,
                batch_depths: BTreeMap::new(),
            };
            audit.anchor_lookup(root, &object, before)?
        };
        if old.as_ref() == Some(value) {
            continue;
        }
        if old.is_none() {
            after = after.checked_add(1).ok_or(PackageError::Limit)?;
        }
        if after > limits.max_entries as u64 {
            return Err(PackageError::Limit);
        }
        changes.push(json!({"object":value["object"],"change":{"kind":"put","membership":value["membership"],"physical":value["physical"]}}));
    }
    Ok(PutBasis {
        base,
        base_count,
        index,
        batches,
        sequence,
        before,
        after,
        changes,
    })
}
