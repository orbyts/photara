//! Independent typed-edge audit of repeatable v2 retained authority inventories.
use super::selected::{json_ref, schema};
use super::tree::{fields, number};
use super::{Locator, Membership, PhysicalRecords, TreeLimits};
use crate::package::{ObjectRef, PackageError, PackageUuid};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
type Result<T> = std::result::Result<T, PackageError>;
fn check(v: bool) -> Result<()> {
    if v {
        Ok(())
    } else {
        Err(PackageError::Integrity)
    }
}
fn reference(v: &Value) -> Result<ObjectRef> {
    let bytes = photara_core::canonical_json(v).map_err(|_| PackageError::Record)?;
    Ok(ObjectRef {
        kind: crate::package::ObjectKind::Json,
        sha256: crate::package::digest(&bytes),
        byte_length: crate::package::DecimalU64::parse(&bytes.len().to_string())?,
    })
}
fn membership(v: &Value) -> Result<Membership> {
    match v.as_str() {
        Some("semantic") => Ok(Membership::Semantic),
        Some("ownership") => Ok(Membership::Ownership),
        _ => Err(PackageError::Record),
    }
}
fn kind(v: &Value) -> Membership {
    if v["schema"]["id"]
        .as_str()
        .is_some_and(|id| id.starts_with("photara.storage."))
    {
        Membership::Ownership
    } else {
        Membership::Semantic
    }
}
#[derive(Clone)]
struct Node {
    entries: BTreeMap<ObjectRef, Membership>,
    children: Vec<ObjectRef>,
    first: Option<ObjectRef>,
    last: Option<ObjectRef>,
    count: usize,
    height: usize,
}
#[derive(Clone, PartialEq)]
enum Part {
    Tree(ObjectRef),
    Entry(ObjectRef, Membership),
}
struct Audit<'a, 'p, P> {
    loc: &'a Locator<'p, P>,
    project: PackageUuid,
    limits: TreeLimits,
    cache: BTreeMap<ObjectRef, Node>,
    visiting: BTreeSet<ObjectRef>,
}
impl<P: PhysicalRecords> Audit<'_, '_, P> {
    fn tree(&mut self, r: &ObjectRef, depth: usize) -> Result<Node> {
        if depth > self.limits.max_depth {
            return Err(PackageError::Limit);
        }
        if let Some(node) = self.cache.get(r) {
            if depth.checked_add(node.height).ok_or(PackageError::Limit)? > self.limits.max_depth {
                return Err(PackageError::Limit);
            }
            return Ok(node.clone());
        }
        check(self.visiting.insert(r.clone()))?;
        if self
            .cache
            .len()
            .checked_add(self.visiting.len())
            .ok_or(PackageError::Limit)?
            > self.limits.max_pages
        {
            return Err(PackageError::Limit);
        }
        let body = self.loc.json(r, Membership::Ownership)?;
        let leaf = body["schema"]["id"] == "photara.storage.retained-control-inventory-leaf";
        schema(
            &body,
            if leaf {
                "photara.storage.retained-control-inventory-leaf"
            } else {
                "photara.storage.retained-control-inventory-branch"
            },
            1,
            &["count", if leaf { "entries" } else { "children" }],
            self.project,
        )?;
        check(body["extensions"] == json!({}))?;
        let mut node = Node {
            entries: BTreeMap::new(),
            children: Vec::new(),
            first: None,
            last: None,
            count: 0,
            height: 0,
        };
        if leaf {
            let rows = body["entries"].as_array().ok_or(PackageError::Record)?;
            if rows.len() > self.limits.max_leaf_entries {
                return Err(PackageError::Limit);
            }
            node.entries = self.rows(rows)?;
            node.first = node.entries.first_key_value().map(|(r, _)| r.clone());
            node.last = node.entries.last_key_value().map(|(r, _)| r.clone());
            node.count = node.entries.len();
        } else {
            let rows = body["children"].as_array().ok_or(PackageError::Record)?;
            check(rows.len() >= 2)?;
            if rows.len() > self.limits.max_branch_children {
                return Err(PackageError::Limit);
            }
            for row in rows {
                fields(row, &["first", "last", "count", "child"])?;
                let child_ref = json_ref(&row["child"])?;
                let child = self.tree(&child_ref, depth + 1)?;
                let first = child.first.as_ref().ok_or(PackageError::Integrity)?;
                let last = child.last.as_ref().ok_or(PackageError::Integrity)?;
                check(
                    json_ref(&row["first"])? == *first
                        && json_ref(&row["last"])? == *last
                        && number(&row["count"])? == child.count as u64,
                )?;
                if let Some(previous) = &node.last {
                    check(previous < first)?;
                }
                if node.first.is_none() {
                    node.first = Some(first.clone());
                }
                node.last = Some(last.clone());
                node.count = node
                    .count
                    .checked_add(child.count)
                    .ok_or(PackageError::Limit)?;
                node.height = node
                    .height
                    .max(child.height.checked_add(1).ok_or(PackageError::Limit)?);
                node.children.push(child_ref);
            }
        }
        check(number(&body["count"])? == node.count as u64)?;
        if node.count > self.limits.max_entries {
            return Err(PackageError::Limit);
        }
        self.visiting.remove(r);
        self.cache.insert(r.clone(), node.clone());
        Ok(node)
    }
    fn expand(&self, queue: &mut VecDeque<Part>) -> Result<()> {
        let Some(Part::Tree(r)) = queue.pop_front() else {
            return Err(PackageError::Integrity);
        };
        let node = self.cache.get(&r).ok_or(PackageError::Integrity)?;
        if node.children.is_empty() {
            for (r, m) in node.entries.iter().rev() {
                queue.push_front(Part::Entry(r.clone(), *m));
            }
        } else {
            for r in node.children.iter().rev() {
                queue.push_front(Part::Tree(r.clone()));
            }
        }
        Ok(())
    }
    fn lookup(&self, root: &ObjectRef, key: &ObjectRef) -> Result<Membership> {
        let node = self.cache.get(root).ok_or(PackageError::Integrity)?;
        if node.children.is_empty() {
            return node
                .entries
                .get(key)
                .copied()
                .ok_or(PackageError::Integrity);
        }
        for child in &node.children {
            let n = self.cache.get(child).ok_or(PackageError::Integrity)?;
            if n.first.as_ref().is_some_and(|r| r <= key)
                && n.last.as_ref().is_some_and(|r| r >= key)
            {
                return self.lookup(child, key);
            }
        }
        Err(PackageError::Integrity)
    }
    fn union(
        &self,
        mut old: VecDeque<Part>,
        root: &ObjectRef,
        additions: &BTreeMap<ObjectRef, Membership>,
    ) -> Result<()> {
        for (r, m) in additions {
            check(self.lookup(root, r)? == *m)?;
        }
        let mut new = VecDeque::from([Part::Tree(root.clone())]);
        loop {
            if old.front().is_some() && old.front() == new.front() {
                old.pop_front();
                new.pop_front();
                continue;
            }
            let expand_old = matches!(old.front(), Some(Part::Tree(_)));
            let expand_new = matches!(new.front(), Some(Part::Tree(_)));
            if expand_old {
                self.expand(&mut old)?;
            }
            if expand_new {
                self.expand(&mut new)?;
            }
            if expand_old || expand_new {
                continue;
            }
            match (old.front(), new.front()) {
                (None, None) => return Ok(()),
                (Some(Part::Entry(a, _)), Some(Part::Entry(b, _))) if a <= b => {
                    return Err(PackageError::Integrity);
                }
                (_, Some(Part::Entry(r, m))) => {
                    check(additions.get(r) == Some(m))?;
                    new.pop_front();
                }
                _ => return Err(PackageError::Integrity),
            }
        }
    }
    fn entries(&self, root: &ObjectRef) -> Result<BTreeMap<ObjectRef, Membership>> {
        let mut queue = VecDeque::from([Part::Tree(root.clone())]);
        let mut out = BTreeMap::new();
        while let Some(part) = queue.front() {
            match part {
                Part::Tree(_) => self.expand(&mut queue)?,
                Part::Entry(r, m) => {
                    check(out.insert(r.clone(), *m).is_none())?;
                    queue.pop_front();
                }
            }
        }
        Ok(out)
    }
    fn rows(&self, rows: &[Value]) -> Result<BTreeMap<ObjectRef, Membership>> {
        if rows.len() > self.limits.max_entries {
            return Err(PackageError::Limit);
        }
        let mut out = BTreeMap::new();
        for row in rows {
            fields(row, &["record", "membership"])?;
            let r = json_ref(&row["record"])?;
            let m = membership(&row["membership"])?;
            if let Some((previous, _)) = out.last_key_value() {
                check(previous < &r)?;
            }
            let body = self.loc.json(&r, m)?;
            check(kind(&body) == m)?;
            out.insert(r, m);
        }
        Ok(out)
    }
    fn descriptor(&mut self, original: &Value) -> Result<ObjectRef> {
        if !cfg!(test) && original["original_codec"] == "photara.codec.ps2-repeatable-admission-v4"
        {
            return Err(PackageError::UnsupportedVersion);
        }
        let descriptor = &original["payload_recipe"]["retained_control_inventory"];
        fields(descriptor, &["base", "additions", "result"])?;
        check(
            original["payload_recipe"]
                .get("retained_controls")
                .is_none(),
        )?;
        let hold = self.loc.json(
            &json_ref(&original["old"]["envelope"]["holds"])?,
            Membership::Ownership,
        )?;
        let hold_entries = hold["entries"].as_array().ok_or(PackageError::Record)?;
        check(hold_entries.len() <= 1)?;
        let base = &descriptor["base"];
        let expected = if hold_entries.is_empty() {
            fields(base, &["kind"])?;
            check(base["kind"] == "empty")?;
            VecDeque::new()
        } else {
            check(hold["remaining"] == "0")?;
            let prior_ref = json_ref(&hold_entries[0]["original"])?;
            let prior = self.loc.json(&prior_ref, Membership::Ownership)?;
            let phase = self
                .loc
                .json(&json_ref(&hold_entries[0]["phase"])?, Membership::Ownership)?;
            check(
                phase["stage"] == "clean"
                    && phase["remaining"] == "0"
                    && json_ref(&phase["original"])? == prior_ref,
            )?;
            match prior["original_codec"].as_str() {
                Some("photara.codec.ps2-repeatable-admission-v1") => {
                    fields(base, &["kind", "original"])?;
                    check(base["kind"] == "legacy" && json_ref(&base["original"])? == prior_ref)?;
                    self.rows(
                        prior["payload_recipe"]["retained_controls"]
                            .as_array()
                            .ok_or(PackageError::Record)?,
                    )?
                    .into_iter()
                    .map(|(r, m)| Part::Entry(r, m))
                    .collect()
                }
                Some(
                    "photara.codec.ps2-repeatable-admission-v2"
                    | "photara.codec.ps2-repeatable-admission-v3"
                    | "photara.codec.ps2-repeatable-admission-v4",
                ) => {
                    fields(base, &["kind", "root"])?;
                    check(
                        base["kind"] == "tree"
                            && base["root"]
                                == prior["payload_recipe"]["retained_control_inventory"]["result"],
                    )?;
                    {
                        let root = json_ref(&base["root"])?;
                        self.tree(&root, 0)?;
                        VecDeque::from([Part::Tree(root)])
                    }
                }
                _ => return Err(PackageError::Record),
            }
        };
        let additions = self.rows(
            descriptor["additions"]
                .as_array()
                .ok_or(PackageError::Record)?,
        )?;
        let mut actual = BTreeMap::new();
        if !hold_entries.is_empty() {
            let overlay = &original["old"]["overlay"];
            check(
                reference(overlay)?
                    == json_ref(&original["old"]["commit"]["root_set"]["inventory"])?,
            )?;
            for r in overlay["controls"].as_array().ok_or(PackageError::Record)? {
                let r = json_ref(r)?;
                // Exact additions determine the role, then actual bytes must agree.
                let m = *additions.get(&r).ok_or(PackageError::Integrity)?;
                let value = self.loc.json(&r, m)?;
                check(kind(&value) == m)?;
                actual.insert(r, m);
            }
        }
        check(actual == additions)?;
        let result = json_ref(&descriptor["result"])?;
        self.tree(&result, 0)?;
        self.union(expected, &result, &additions)?;
        Ok(result)
    }
}
pub(super) fn inspect<P: PhysicalRecords>(
    loc: &Locator<'_, P>,
    original: &Value,
    expected: &BTreeMap<ObjectRef, Membership>,
    project: PackageUuid,
    limits: TreeLimits,
) -> Result<()> {
    if !matches!(
        original["original_codec"].as_str(),
        Some(
            "photara.codec.ps2-repeatable-admission-v2"
                | "photara.codec.ps2-repeatable-admission-v3"
                | "photara.codec.ps2-repeatable-admission-v4"
        )
    ) {
        return Ok(());
    }
    let mut audit = Audit {
        loc,
        project,
        limits,
        cache: BTreeMap::new(),
        visiting: BTreeSet::new(),
    };
    let root = audit.descriptor(original)?;
    let current = audit.entries(&root)?;
    let mut all = current.clone();
    for (r, m) in &current {
        let body = loc.json(r, *m)?;
        if matches!(
            body["original_codec"].as_str(),
            Some(
                "photara.codec.ps2-repeatable-admission-v2"
                    | "photara.codec.ps2-repeatable-admission-v3"
                    | "photara.codec.ps2-repeatable-admission-v4"
            )
        ) {
            audit.descriptor(&body)?;
        }
    }
    // Each physical node is stored once, independent of the number of historical roots.
    // Its entries must still be members of the current authenticated inventory.
    for node in audit.cache.values() {
        check(node.entries.iter().all(|(r, m)| current.get(r) == Some(m)))?;
    }
    for node in audit.cache.keys() {
        check(all.insert(node.clone(), Membership::Ownership).is_none())?;
    }
    check(all == *expected)
}

#[cfg(test)]
#[expect(
    clippy::too_many_lines,
    reason = "One packed fixture exercises typed-edge and cached-depth refusal together"
)]
pub(super) fn audit_regressions() {
    use super::{Arena, FrameKind, FrameLimits, PackedAllocation, PhysicalRef, encode_frame};
    let project = PackageUuid::parse("10000000-0000-4000-8000-000000000001").unwrap();
    let allocation = PackageUuid::parse("10000000-0000-4000-8000-000000000002").unwrap();
    let frame_limits = FrameLimits {
        json: crate::package::JsonLimits {
            max_bytes: 1_000_000,
            max_depth: 64,
            max_members: 1024,
            max_array_elements: 1024,
        },
        max_allocation_bytes: 1_000_000,
        max_frames: 1024,
    };
    let limits = TreeLimits {
        max_depth: 2,
        max_pages: 128,
        max_entries: 128,
        max_leaf_entries: 128,
        max_branch_children: 8,
    };
    let mut bytes = Vec::new();
    let mut rows = Vec::new();
    let mut add = |body: Value, membership: Membership| {
        let canonical = photara_core::canonical_json(&body).unwrap();
        let object = reference(&body).unwrap();
        let frame = encode_frame(
            Arena::Data,
            if membership == Membership::Semantic {
                FrameKind::Semantic
            } else {
                FrameKind::Ownership
            },
            &canonical,
            frame_limits,
        )
        .unwrap();
        let physical: PhysicalRef = serde_json::from_value(json!({"allocation_id":allocation,"arena":"data","offset":bytes.len().to_string(),"byte_length":frame.len().to_string(),"record_sha256":crate::package::digest(&frame)})).unwrap();
        bytes.extend(frame);
        rows.push(json!({"object":object,"membership":if membership==Membership::Semantic {"semantic"} else {"ownership"},"physical":physical}));
        object
    };
    let mut controls = (0..4)
        .map(|n| add(json!({"payload":n}), Membership::Semantic))
        .collect::<Vec<_>>();
    controls.sort();
    let leaf = |r: &ObjectRef| json!({"schema":{"id":"photara.storage.retained-control-inventory-leaf","version":1},"project_id":project,"extensions":{},"count":"1","entries":[{"record":r,"membership":"semantic"}]});
    let leaves = controls
        .iter()
        .map(|r| add(leaf(r), Membership::Ownership))
        .collect::<Vec<_>>();
    let child =
        |n: usize| json!({"first":controls[n],"last":controls[n],"count":"1","child":leaves[n]});
    let branch = json!({"schema":{"id":"photara.storage.retained-control-inventory-branch","version":1},"project_id":project,"extensions":{},"count":"2","children":[child(0),child(1)]});
    let root = add(branch.clone(), Membership::Ownership);
    let mut false_count = branch.clone();
    false_count["count"] = json!("3");
    let false_count = add(false_count, Membership::Ownership);
    let mut missing = branch.clone();
    missing["children"][0]["child"]["sha256"] = json!("f".repeat(64));
    let missing = add(missing, Membership::Ownership);
    let mut wrong_member = leaf(&controls[0]);
    wrong_member["entries"][0]["membership"] = json!("ownership");
    let wrong_member = add(wrong_member, Membership::Ownership);
    let mut extension = branch;
    extension["extensions"] = json!({"unknown":true});
    let extension = add(extension, Membership::Ownership);
    rows.sort_by_key(|v| json_ref(&v["object"]).unwrap());
    let locator_body = json!({"schema":{"id":"photara.storage.locator-leaf","version":1},"project_id":project,"extensions":{},"count":rows.len().to_string(),"entries":rows});
    let frame = encode_frame(
        Arena::Metadata,
        FrameKind::Physical,
        &photara_core::canonical_json(&locator_body).unwrap(),
        frame_limits,
    )
    .unwrap();
    let metadata = PackageUuid::parse("10000000-0000-4000-8000-000000000003").unwrap();
    let locator_ref = serde_json::from_value(json!({"allocation_id":metadata,"arena":"metadata","offset":"0","byte_length":frame.len().to_string(),"record_sha256":crate::package::digest(&frame)})).unwrap();
    let provider = BTreeMap::from([
        (
            allocation,
            PackedAllocation::open(allocation, Arena::Data, &bytes, frame_limits).unwrap(),
        ),
        (
            metadata,
            PackedAllocation::open(metadata, Arena::Metadata, &frame, frame_limits).unwrap(),
        ),
    ]);
    let loc = Locator::new(&provider, project, locator_ref, limits);
    let mut audit = Audit {
        loc: &loc,
        project,
        limits,
        cache: BTreeMap::new(),
        visiting: BTreeSet::new(),
    };
    assert_eq!(audit.tree(&root, 0).unwrap().height, 1);
    assert!(audit.tree(&root, 1).is_ok());
    assert!(matches!(audit.tree(&root, 2), Err(PackageError::Limit)));
    assert_eq!(audit.cache.len(), 3);
    assert_eq!(
        audit
            .cache
            .values()
            .map(|node| node.entries.len())
            .sum::<usize>(),
        2
    );
    for r in [false_count, missing, wrong_member, extension] {
        let mut audit = Audit {
            loc: &loc,
            project,
            limits,
            cache: BTreeMap::new(),
            visiting: BTreeSet::new(),
        };
        assert!(audit.tree(&r, 0).is_err());
    }
}
