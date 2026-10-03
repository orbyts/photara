//! Authenticated physical-tree traversal with scoped reads and a separate full audit.
use super::super::{DecimalU64, ObjectKind, ObjectRef, PackageError, PackageUuid};
use super::{FrameKind, PhysicalRef};
use photara_core::contracts::schema::QualifiedName;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// A provider returns a canonical, digest-checked record at an exact frame boundary.
/// Implementations must enforce allocation identity and tag; a path or byte slice
/// by itself is not a frame-boundary proof. `PackedAllocation::resolve` supplies
/// this contract for an already indexed immutable allocation.
pub trait PhysicalRecords {
    /// # Errors
    /// Refuses missing, malformed, substituted or incorrectly typed frames.
    fn resolve(&self, reference: &PhysicalRef, kind: FrameKind) -> Result<Value, PackageError>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalTreeKind {
    Locator,
    RootPlacement,
}
impl PhysicalTreeKind {
    fn name(self) -> &'static str {
        match self {
            Self::Locator => "locator",
            Self::RootPlacement => "root-placement",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Locator => "object",
            Self::RootPlacement => "root",
        }
    }
}
/// Caller work budgets, not format or product limits.
#[derive(Clone, Copy, Debug)]
pub struct TreeLimits {
    pub max_depth: usize,
    pub max_pages: usize,
    pub max_entries: usize,
    pub max_leaf_entries: usize,
    pub max_branch_children: usize,
}
struct Summary {
    first: Option<ObjectRef>,
    last: Option<ObjectRef>,
    count: u64,
}
struct Page {
    summary: Summary,
    body: Body,
}
enum Body {
    Leaf(Vec<Value>),
    Branch(Vec<(Summary, PhysicalRef)>),
}

pub struct PhysicalTreeAudit {
    pub entries: Vec<Value>,
    pub pages: Vec<PhysicalRef>,
}

pub struct PhysicalTree<'a, P> {
    provider: &'a P,
    project: PackageUuid,
    kind: PhysicalTreeKind,
    limits: TreeLimits,
}
impl<'a, P: PhysicalRecords> PhysicalTree<'a, P> {
    #[must_use]
    pub fn new(
        provider: &'a P,
        project: PackageUuid,
        kind: PhysicalTreeKind,
        limits: TreeLimits,
    ) -> Self {
        Self {
            provider,
            project,
            kind,
            limits,
        }
    }
    /// Authenticates the selected path only. Unaccessed descendants have no
    /// integrity claim; this result cannot grant imported-package write admission.
    /// # Errors
    /// Refuses malformed schema, ordering, range/count, alias/cycle or work bounds.
    pub fn lookup(
        &self,
        root: &PhysicalRef,
        key: &ObjectRef,
    ) -> Result<Option<Value>, PackageError> {
        validate_key(key, self.kind)?;
        let mut reference = root.clone();
        let mut expected = None;
        let mut seen = BTreeSet::new();
        for _ in 0..self.limits.max_depth {
            self.visit(&reference, &mut seen)?;
            let page = self.page(&reference)?;
            if let Some(summary) = expected {
                check_summary(&summary, &page.summary)?;
            }
            match page.body {
                Body::Leaf(entries) => {
                    for entry in entries {
                        if self.entry_key(&entry)? == *key {
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
    /// Traverses every page and checks each child summary against its actual
    /// authenticated descendants. This is a tree audit, not a whole-package audit.
    /// # Errors
    /// Refuses malformed/missing descendants, summary mismatches and budget limits.
    pub fn audit(&self, root: &PhysicalRef) -> Result<Vec<Value>, PackageError> {
        Ok(self.audit_with_pages(root)?.entries)
    }
    /// Full tree audit including directly authenticated implementation pages.
    /// # Errors
    /// Uses the same checks and budgets as `audit`.
    pub fn audit_with_pages(&self, root: &PhysicalRef) -> Result<PhysicalTreeAudit, PackageError> {
        let mut seen = BTreeSet::new();
        let (_, entries) = self.audit_at(root, 0, &mut seen)?;
        let pages = seen
            .into_iter()
            .map(|bytes| serde_json::from_slice(&bytes).map_err(|_| PackageError::Record))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PhysicalTreeAudit { entries, pages })
    }
    fn audit_at(
        &self,
        reference: &PhysicalRef,
        depth: usize,
        seen: &mut BTreeSet<Vec<u8>>,
    ) -> Result<(Summary, Vec<Value>), PackageError> {
        if depth >= self.limits.max_depth {
            return Err(PackageError::Limit);
        }
        self.visit(reference, seen)?;
        let page = self.page(reference)?;
        let mut entries = Vec::new();
        match page.body {
            Body::Leaf(values) => entries = values,
            Body::Branch(children) => {
                for (summary, child) in children {
                    let (actual, values) = self.audit_at(&child, depth + 1, seen)?;
                    check_summary(&summary, &actual)?;
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
        reference: &PhysicalRef,
        seen: &mut BTreeSet<Vec<u8>>,
    ) -> Result<(), PackageError> {
        if seen.len() >= self.limits.max_pages {
            return Err(PackageError::Limit);
        }
        let key = photara_core::canonical_json(
            &serde_json::to_value(reference).map_err(|_| PackageError::Record)?,
        )
        .map_err(|_| PackageError::Record)?;
        if !seen.insert(key) {
            return Err(PackageError::Integrity);
        }
        Ok(())
    }
    fn entry_key(&self, value: &Value) -> Result<ObjectRef, PackageError> {
        match self.kind {
            PhysicalTreeKind::Locator => {
                fields(value, &["object", "membership", "physical"])?;
                super::locator::validate_entry(value)?;
            }
            PhysicalTreeKind::RootPlacement => {
                fields(value, &["root", "root_id", "locator", "ownership"])?;
                let id: PackageUuid = serde_json::from_value(value["root_id"].clone())
                    .map_err(|_| PackageError::Record)?;
                let locator: PhysicalRef = serde_json::from_value(value["locator"].clone())
                    .map_err(|_| PackageError::Record)?;
                let ownership: ObjectRef = serde_json::from_value(value["ownership"].clone())
                    .map_err(|_| PackageError::Record)?;
                if id.as_uuid().is_nil()
                    || locator.allocation_id.as_uuid().is_nil()
                    || locator.arena != super::Arena::Metadata
                    || ownership.kind != ObjectKind::Json
                    || ownership.byte_length.get() == 0
                {
                    return Err(PackageError::Record);
                }
            }
        }
        let key: ObjectRef = serde_json::from_value(value[self.kind.key()].clone())
            .map_err(|_| PackageError::Record)?;
        validate_key(&key, self.kind)?;
        Ok(key)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "Keep exact frozen field and identity checks in auditable protocol order"
    )]
    fn page(&self, reference: &PhysicalRef) -> Result<Page, PackageError> {
        let value = self.provider.resolve(reference, FrameKind::Physical)?;
        let leaf_name = format!("photara.storage.{}-leaf", self.kind.name());
        let branch_name = format!("photara.storage.{}-branch", self.kind.name());
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
            let mut keys = Vec::with_capacity(entries.len());
            for entry in entries {
                keys.push(self.entry_key(entry)?);
            }
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
        let mut parsed = Vec::with_capacity(children.len());
        let mut total = 0u64;
        let mut previous = None;
        for child in children {
            fields(child, &["first", "last", "count", "child"])?;
            let first: ObjectRef =
                serde_json::from_value(child["first"].clone()).map_err(|_| PackageError::Record)?;
            let last: ObjectRef =
                serde_json::from_value(child["last"].clone()).map_err(|_| PackageError::Record)?;
            validate_key(&first, self.kind)?;
            validate_key(&last, self.kind)?;
            let n = number(&child["count"])?;
            if n == 0 || first > last || previous.as_ref().is_some_and(|p| p >= &first) {
                return Err(PackageError::Integrity);
            }
            previous = Some(last.clone());
            total = total.checked_add(n).ok_or(PackageError::Limit)?;
            let physical =
                serde_json::from_value(child["child"].clone()).map_err(|_| PackageError::Record)?;
            parsed.push((
                Summary {
                    first: Some(first),
                    last: Some(last),
                    count: n,
                },
                physical,
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
fn check_summary(expected: &Summary, actual: &Summary) -> Result<(), PackageError> {
    if expected.first != actual.first
        || expected.last != actual.last
        || expected.count != actual.count
    {
        Err(PackageError::Integrity)
    } else {
        Ok(())
    }
}
fn validate_key(key: &ObjectRef, kind: PhysicalTreeKind) -> Result<(), PackageError> {
    if (key.kind == ObjectKind::Json && key.byte_length.get() == 0)
        || (kind == PhysicalTreeKind::RootPlacement && key.kind != ObjectKind::Json)
    {
        Err(PackageError::Record)
    } else {
        Ok(())
    }
}
pub(super) fn fields(value: &Value, names: &[&str]) -> Result<(), PackageError> {
    let object = value.as_object().ok_or(PackageError::Record)?;
    if object.len() != names.len() || names.iter().any(|k| !object.contains_key(*k)) {
        Err(PackageError::Record)
    } else {
        Ok(())
    }
}
pub(super) fn number(value: &Value) -> Result<u64, PackageError> {
    Ok(DecimalU64::parse(value.as_str().ok_or(PackageError::Record)?)?.get())
}

impl PhysicalRecords for BTreeMap<PackageUuid, super::PackedAllocation<'_>> {
    fn resolve(&self, reference: &PhysicalRef, kind: FrameKind) -> Result<Value, PackageError> {
        self.get(&reference.allocation_id)
            .ok_or(PackageError::Integrity)?
            .resolve(reference, kind)
    }
}
