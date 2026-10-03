//! Frozen semantic/ownership tree traversal. Entry semantics are validated by
//! their owning schema, separately from this authenticated tree structure.
use super::super::{ObjectKind, ObjectRef, PackageError, PackageUuid};
use super::tree::{fields, number};
use super::{Locator, Membership, PhysicalRecords, TreeLimits};
use photara_core::contracts::schema::QualifiedName;
use serde_json::Value;
use std::collections::BTreeSet;

pub trait LogicalRecords {
    /// # Errors
    /// Returns only canonical, exact-digest JSON in the requested membership.
    fn resolve(&self, object: &ObjectRef, membership: Membership) -> Result<Value, PackageError>;
}
impl<P: PhysicalRecords> LogicalRecords for Locator<'_, P> {
    fn resolve(&self, object: &ObjectRef, membership: Membership) -> Result<Value, PackageError> {
        self.json(object, membership)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicalTreeKind {
    Inventory,
    OperationId,
    OperationOrdinal,
    Ownership,
    RetainedRoot,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LogicalKey {
    Object(ObjectRef),
    Id(PackageUuid),
    Ordinal(u64),
}
impl LogicalTreeKind {
    fn name(self) -> &'static str {
        match self {
            Self::Inventory => "photara.package.inventory",
            Self::OperationId => "photara.package.operation-id",
            Self::OperationOrdinal => "photara.package.operation-ordinal",
            Self::Ownership => "photara.storage.ownership",
            Self::RetainedRoot => "photara.package.retained-root",
        }
    }
    fn membership(self) -> Membership {
        if self == Self::Ownership {
            Membership::Ownership
        } else {
            Membership::Semantic
        }
    }
    fn entry_key(self, value: &Value) -> Result<LogicalKey, PackageError> {
        self.summary_key(match self {
            Self::Inventory => value,
            Self::OperationId => &value["operation_id"],
            Self::OperationOrdinal => &value["acceptance_ordinal"],
            Self::Ownership => &value["allocation_id"],
            Self::RetainedRoot => &value["pin_id"],
        })
    }
    fn summary_key(self, value: &Value) -> Result<LogicalKey, PackageError> {
        Ok(match self {
            Self::Inventory => {
                let object: ObjectRef =
                    serde_json::from_value(value.clone()).map_err(|_| PackageError::Record)?;
                if object.kind == ObjectKind::Json && object.byte_length.get() == 0 {
                    return Err(PackageError::Record);
                }
                LogicalKey::Object(object)
            }
            Self::OperationOrdinal => LogicalKey::Ordinal(number(value)?),
            _ => {
                let id = PackageUuid::parse(value.as_str().ok_or(PackageError::Record)?)?;
                if id.as_uuid().is_nil() {
                    return Err(PackageError::Record);
                }
                LogicalKey::Id(id)
            }
        })
    }
    fn accepts(self, key: &LogicalKey) -> bool {
        match (self, key) {
            (Self::Inventory, LogicalKey::Object(o)) => {
                o.kind != ObjectKind::Json || o.byte_length.get() > 0
            }
            (Self::OperationOrdinal, LogicalKey::Ordinal(_)) => true,
            (Self::OperationId | Self::Ownership | Self::RetainedRoot, LogicalKey::Id(id)) => {
                !id.as_uuid().is_nil()
            }
            _ => false,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Summary {
    first: Option<LogicalKey>,
    last: Option<LogicalKey>,
    count: u64,
}
enum Body {
    Leaf(Vec<Value>),
    Branch(Vec<(Summary, ObjectRef)>),
}
struct Page {
    summary: Summary,
    body: Body,
}

/// Full tree structure only; does not certify an entire package or leaf semantics.
pub struct LogicalTreeAudit {
    pub entries: Vec<Value>,
    pub nodes: BTreeSet<ObjectRef>,
}
pub struct LogicalTree<'a, P> {
    provider: &'a P,
    project: PackageUuid,
    kind: LogicalTreeKind,
    limits: TreeLimits,
}
impl<'a, P: LogicalRecords> LogicalTree<'a, P> {
    #[must_use]
    pub fn new(
        provider: &'a P,
        project: PackageUuid,
        kind: LogicalTreeKind,
        limits: TreeLimits,
    ) -> Self {
        Self {
            provider,
            project,
            kind,
            limits,
        }
    }
    /// Authenticates one access path. Unaccessed descendants remain unaudited.
    /// # Errors
    /// Refuses wrong key domain, schema/membership, ordering, summaries or budgets.
    pub fn lookup(
        &self,
        root: &ObjectRef,
        key: &LogicalKey,
    ) -> Result<Option<Value>, PackageError> {
        if !self.kind.accepts(key) {
            return Err(PackageError::Record);
        }
        let mut reference = root.clone();
        let mut expected = None;
        let mut nodes = BTreeSet::new();
        for _ in 0..self.limits.max_depth {
            self.visit(&reference, &mut nodes)?;
            let page = self.page(&reference)?;
            if expected.as_ref().is_some_and(|s| s != &page.summary) {
                return Err(PackageError::Integrity);
            }
            match page.body {
                Body::Leaf(entries) => {
                    for entry in entries {
                        if self.kind.entry_key(&entry)? == *key {
                            return Ok(Some(entry));
                        }
                    }
                    return Ok(None);
                }
                Body::Branch(children) => {
                    if let Some((summary, child)) = children.into_iter().find(|(s, _)| {
                        s.first.as_ref().is_some_and(|f| f <= key)
                            && s.last.as_ref().is_some_and(|l| key <= l)
                    }) {
                        reference = child;
                        expected = Some(summary);
                    } else {
                        return Ok(None);
                    }
                }
            }
        }
        Err(PackageError::Limit)
    }
    /// # Errors
    /// Checks every descendant, exact child summaries and disjoint ordered ranges;
    /// repeated nodes, missing content and work-budget exhaustion refuse.
    pub fn audit(&self, root: &ObjectRef) -> Result<LogicalTreeAudit, PackageError> {
        let mut nodes = BTreeSet::new();
        let (_, entries) = self.audit_at(root, 0, &mut nodes)?;
        Ok(LogicalTreeAudit { entries, nodes })
    }
    fn audit_at(
        &self,
        root: &ObjectRef,
        depth: usize,
        nodes: &mut BTreeSet<ObjectRef>,
    ) -> Result<(Summary, Vec<Value>), PackageError> {
        if depth >= self.limits.max_depth {
            return Err(PackageError::Limit);
        }
        self.visit(root, nodes)?;
        let page = self.page(root)?;
        let mut entries = Vec::new();
        match page.body {
            Body::Leaf(values) => entries = values,
            Body::Branch(children) => {
                for (expected, child) in children {
                    let (actual, values) = self.audit_at(&child, depth + 1, nodes)?;
                    if expected != actual {
                        return Err(PackageError::Integrity);
                    }
                    if entries
                        .len()
                        .checked_add(values.len())
                        .ok_or(PackageError::Limit)?
                        > self.limits.max_entries
                    {
                        return Err(PackageError::Limit);
                    }
                    entries.extend(values);
                }
            }
        }
        Ok((page.summary, entries))
    }
    fn visit(
        &self,
        object: &ObjectRef,
        nodes: &mut BTreeSet<ObjectRef>,
    ) -> Result<(), PackageError> {
        if object.kind != ObjectKind::Json || object.byte_length.get() == 0 {
            return Err(PackageError::Record);
        }
        if nodes.len() >= self.limits.max_pages {
            return Err(PackageError::Limit);
        }
        if !nodes.insert(object.clone()) {
            return Err(PackageError::Integrity);
        }
        Ok(())
    }
    #[allow(
        clippy::too_many_lines,
        reason = "One schema-directed page pass checks exact fields, ranges and aggregate"
    )]
    fn page(&self, object: &ObjectRef) -> Result<Page, PackageError> {
        let value = self.provider.resolve(object, self.kind.membership())?;
        let leaf_name = format!("{}-leaf", self.kind.name());
        let branch_name = format!("{}-branch", self.kind.name());
        let leaf = value["schema"]["id"] == leaf_name;
        fields(
            &value,
            &[
                "schema",
                "project_id",
                "extensions",
                "count",
                if leaf { "entries" } else { "children" },
            ],
        )?;
        fields(&value["schema"], &["id", "version"])?;
        if value["schema"]["id"]
            != if leaf {
                leaf_name.as_str()
            } else {
                branch_name.as_str()
            }
            || value["schema"]["version"] != 1
            || value["project_id"] != self.project.to_string()
            || self.project.as_uuid().is_nil()
        {
            return Err(PackageError::UnsupportedVersion);
        }
        let extensions = value["extensions"]
            .as_object()
            .ok_or(PackageError::Record)?;
        if extensions
            .keys()
            .any(|k| QualifiedName::parse(k.clone()).is_err())
        {
            return Err(PackageError::Record);
        }
        let count = number(&value["count"])?;
        if count > self.limits.max_entries as u64 {
            return Err(PackageError::Limit);
        }
        if leaf {
            let entries = value["entries"].as_array().ok_or(PackageError::Record)?;
            if entries.len() > self.limits.max_leaf_entries {
                return Err(PackageError::Limit);
            }
            if entries.len() as u64 != count {
                return Err(PackageError::Integrity);
            }
            let keys: Vec<_> = entries
                .iter()
                .map(|e| self.kind.entry_key(e))
                .collect::<Result<_, _>>()?;
            if keys.windows(2).any(|p| p[0] >= p[1]) {
                return Err(PackageError::Integrity);
            }
            return Ok(Page {
                summary: Summary {
                    first: keys.first().cloned(),
                    last: keys.last().cloned(),
                    count,
                },
                body: Body::Leaf(entries.clone()),
            });
        }
        let children = value["children"].as_array().ok_or(PackageError::Record)?;
        if children.len() < 2 {
            return Err(PackageError::Record);
        }
        if children.len() > self.limits.max_branch_children {
            return Err(PackageError::Limit);
        }
        let mut parsed = Vec::new();
        let mut total = 0u64;
        let mut previous = None;
        for child in children {
            fields(child, &["first", "last", "count", "child"])?;
            let first = self.kind.summary_key(&child["first"])?;
            let last = self.kind.summary_key(&child["last"])?;
            let n = number(&child["count"])?;
            if n == 0 || first > last || previous.as_ref().is_some_and(|p| p >= &first) {
                return Err(PackageError::Integrity);
            }
            previous = Some(last.clone());
            total = total.checked_add(n).ok_or(PackageError::Limit)?;
            let object: ObjectRef =
                serde_json::from_value(child["child"].clone()).map_err(|_| PackageError::Record)?;
            if object.kind != ObjectKind::Json || object.byte_length.get() == 0 {
                return Err(PackageError::Record);
            }
            parsed.push((
                Summary {
                    first: Some(first),
                    last: Some(last),
                    count: n,
                },
                object,
            ));
        }
        if total != count {
            return Err(PackageError::Integrity);
        }
        Ok(Page {
            summary: Summary {
                first: parsed.first().and_then(|(s, _)| s.first.clone()),
                last: parsed.last().and_then(|(s, _)| s.last.clone()),
                count,
            },
            body: Body::Branch(parsed),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::{DecimalU64, digest};
    use std::collections::BTreeMap;

    struct Records(BTreeMap<ObjectRef, Value>);
    fn reference(value: &Value) -> ObjectRef {
        let bytes = photara_core::canonical_json(value).unwrap();
        ObjectRef {
            kind: ObjectKind::Json,
            byte_length: DecimalU64::parse(&bytes.len().to_string()).unwrap(),
            sha256: digest(&bytes),
        }
    }
    impl LogicalRecords for Records {
        fn resolve(
            &self,
            object: &ObjectRef,
            membership: Membership,
        ) -> Result<Value, PackageError> {
            let value = self.0.get(object).ok_or(PackageError::Integrity)?;
            let expected = if value["schema"]["id"]
                .as_str()
                .unwrap()
                .starts_with("photara.storage.ownership-")
            {
                Membership::Ownership
            } else {
                Membership::Semantic
            };
            if reference(value) != *object || membership != expected {
                return Err(PackageError::Integrity);
            }
            Ok(value.clone())
        }
    }
    fn limits() -> TreeLimits {
        TreeLimits {
            max_depth: 16,
            max_pages: 512,
            max_entries: 4096,
            max_leaf_entries: 4,
            max_branch_children: 4,
        }
    }
    fn corpus() -> Records {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../../../docs/architecture/proposals/ps2/integrated/linked.json"
        ))
        .unwrap();
        Records(
            corpus["records"]
                .as_object()
                .unwrap()
                .values()
                .map(|row| {
                    let value: Value =
                        serde_json::from_str(row["canonical"].as_str().unwrap()).unwrap();
                    (reference(&value), value)
                })
                .collect(),
        )
    }
    #[test]
    fn frozen_five_tree_kinds_audit_and_scoped_lookup_agree() {
        let records = corpus();
        let project = PackageUuid::parse("10000000-0000-4000-8000-000000000001").unwrap();
        for kind in [
            LogicalTreeKind::Inventory,
            LogicalTreeKind::OperationId,
            LogicalTreeKind::OperationOrdinal,
            LogicalTreeKind::Ownership,
            LogicalTreeKind::RetainedRoot,
        ] {
            let tree = LogicalTree::new(&records, project, kind, limits());
            let mut checked = 0;
            for (root, value) in &records.0 {
                if !value["schema"]["id"]
                    .as_str()
                    .unwrap()
                    .starts_with(&format!("{}-", kind.name()))
                {
                    continue;
                }
                let audit = tree.audit(root).unwrap();
                assert_eq!(audit.entries.len() as u64, number(&value["count"]).unwrap());
                assert!(audit.nodes.contains(root));
                for entry in &audit.entries {
                    assert_eq!(
                        tree.lookup(root, &kind.entry_key(entry).unwrap()).unwrap(),
                        Some(entry.clone())
                    );
                }
                checked += 1;
            }
            assert!(checked > 0, "{kind:?}");
        }
    }
    #[test]
    fn coherent_wrong_summaries_missing_children_and_limits_refuse() {
        let mut records = corpus();
        let project = PackageUuid::parse("10000000-0000-4000-8000-000000000001").unwrap();
        let (_, branch) = records
            .0
            .iter()
            .find(|(_, v)| v["schema"]["id"] == "photara.package.inventory-branch")
            .unwrap();
        let branch = branch.clone();
        let root = reference(&branch);
        let mut budget = limits();
        budget.max_pages = 0;
        assert!(matches!(
            LogicalTree::new(&records, project, LogicalTreeKind::Inventory, budget).audit(&root),
            Err(PackageError::Limit)
        ));
        let mut altered = branch.clone();
        altered["children"][0]["count"] =
            Value::String((number(&altered["children"][0]["count"]).unwrap() + 1).to_string());
        altered["count"] = Value::String((number(&altered["count"]).unwrap() + 1).to_string());
        let altered_root = reference(&altered);
        records.0.insert(altered_root.clone(), altered);
        assert!(matches!(
            LogicalTree::new(&records, project, LogicalTreeKind::Inventory, limits())
                .audit(&altered_root),
            Err(PackageError::Integrity)
        ));
        let child: ObjectRef =
            serde_json::from_value(branch["children"][0]["child"].clone()).unwrap();
        records.0.remove(&child);
        assert!(matches!(
            LogicalTree::new(&records, project, LogicalTreeKind::Inventory, limits()).audit(&root),
            Err(PackageError::Integrity)
        ));
    }
}
