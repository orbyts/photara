//! Selected authored/history/recovery/explicit origin policy. Pending is unsupported.
use super::{resources, wire};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use wire::{Key, Result, ensure, fields, key, number, schema, text};
pub(super) struct Origins;
type OriginKey = (String, String);
type Requirements = BTreeMap<String, Value>;

fn uuid(v: &Value) -> Result<&str> {
    let s = text(v)?;
    let id = uuid::Uuid::parse_str(s).map_err(|_| "origin UUID")?;
    ensure(!id.is_nil() && id.to_string() == s, "origin canonical UUID")?;
    Ok(s)
}
fn origin_key(v: &Value) -> Result<OriginKey> {
    fields(v, &["kind", "source_id"])?;
    let kind = text(&v["kind"])?;
    ensure(
        ["history", "recovery", "explicit"].contains(&kind),
        "pending/unknown origin evidence unsupported",
    )?;
    Ok((kind.into(), uuid(&v["source_id"])?.into()))
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    let a = v.as_array().ok_or("origin array")?;
    ensure(a.len() <= 256, "origin array bound")?;
    Ok(a)
}
fn fetched(res: &wire::Resolver, r: &Value, set: &mut BTreeSet<Key>) -> Result<Value> {
    let v = res.get(r, 1)?;
    ensure(
        wire::reference(&v) == *r,
        "origin referenced canonical bytes",
    )?;
    set.insert(key(r)?);
    Ok(v)
}
#[derive(Clone, Copy)]
enum Tree {
    Evidence,
    Requirements,
}
impl Tree {
    fn node(self, leaf: bool) -> &'static str {
        match (self, leaf) {
            (Self::Evidence, true) => "photara.resource.retention-evidence-leaf",
            (Self::Evidence, false) => "photara.resource.retention-evidence-branch",
            (Self::Requirements, true) => "photara.resource.selection-leaf",
            (Self::Requirements, false) => "photara.resource.selection-branch",
        }
    }
    fn boundary(self, v: &Value) -> Result<OriginKey> {
        match self {
            Self::Evidence => origin_key(v),
            Self::Requirements => Ok((String::new(), uuid(v)?.into())),
        }
    }
    fn entry(self, v: &Value) -> Result<OriginKey> {
        fields(
            v,
            match self {
                Self::Evidence => &["key", "evidence"],
                Self::Requirements => &["id", "record"],
            },
        )?;
        self.boundary(
            &v[match self {
                Self::Evidence => "key",
                Self::Requirements => "id",
            }],
        )
    }
}
fn walk(
    res: &wire::Resolver,
    r: &Value,
    kind: Tree,
    nodes: &mut BTreeSet<Key>,
    visited: &mut BTreeSet<Key>,
    depth: usize,
) -> Result<Vec<Value>> {
    ensure(
        depth < 16 && visited.len() < 512 && visited.insert(key(r)?),
        "origin tree alias/depth bound",
    )?;
    let v = fetched(res, r, nodes)?;
    let leaf = v["schema"]["id"] == kind.node(true);
    let mut extras = vec!["count", if leaf { "entries" } else { "children" }];
    if matches!(kind, Tree::Requirements) {
        extras.push("selection_kind");
    }
    schema(&v, kind.node(leaf), 1, &extras)?;
    if matches!(kind, Tree::Requirements) {
        ensure(
            v["selection_kind"] == "requirements",
            "origin requirement tree kind",
        )?;
    }
    let mut entries = Vec::new();
    if leaf {
        let es = array(&v["entries"])?;
        ensure(es.len() <= 4, "origin leaf bound")?;
        entries.clone_from(es);
    } else {
        let children = array(&v["children"])?;
        ensure((2..=4).contains(&children.len()), "origin branch fanout")?;
        for child in children {
            fields(child, &["first", "last", "count", "child"])?;
            let sub = walk(res, &child["child"], kind, nodes, visited, depth + 1)?;
            ensure(!sub.is_empty(), "origin empty child")?;
            ensure(
                kind.boundary(&child["first"])? == kind.entry(&sub[0])?
                    && kind.boundary(&child["last"])?
                        == kind.entry(sub.last().ok_or("origin child")?)?
                    && number(&child["count"])? == sub.len() as u64,
                "origin child summary",
            )?;
            entries.extend(sub);
            ensure(entries.len() <= 4096, "origin tree count bound")?;
        }
    }
    let mut previous = None;
    for entry in &entries {
        let current = kind.entry(entry)?;
        ensure(
            previous.as_ref().is_none_or(|p| p < &current),
            "origin strict tree order",
        )?;
        previous = Some(current);
    }
    ensure(
        number(&v["count"])? == entries.len() as u64,
        "origin tree count",
    )?;
    Ok(entries)
}
fn requirements(
    res: &wire::Resolver,
    r: &Value,
    contribution: &mut wire::Contribution,
) -> Result<Requirements> {
    let entries = walk(
        res,
        r,
        Tree::Requirements,
        &mut contribution.implementation,
        &mut BTreeSet::new(),
        0,
    )?;
    let mut result = BTreeMap::new();
    for e in entries {
        let v = fetched(res, &e["record"], &mut contribution.logical)?;
        schema(
            &v,
            "photara.resource.retention-requirement",
            1,
            &[
                "requirement_id",
                "revision",
                "resource_id",
                "version_id",
                "minimum_qualified_copies",
                "required_backing_ids",
                "required_qualification",
            ],
        )?;
        ensure(
            v["requirement_id"] == e["id"],
            "origin requirement identity",
        )?;
        requirement_scalars(&v)?;
        result.insert(uuid(&e["id"])?.into(), v);
    }
    Ok(result)
}
fn requirement_scalars(v: &Value) -> Result<()> {
    for field in ["requirement_id", "resource_id", "version_id"] {
        uuid(&v[field])?;
    }
    number(&v["revision"])?;
    ensure(
        number(&v["minimum_qualified_copies"])? > 0,
        "origin positive copies",
    )?;
    let q = &v["required_qualification"];
    fields(
        q,
        &[
            "profile_id",
            "minimum_profile_revision",
            "failure_model_sha256",
        ],
    )?;
    uuid(&q["profile_id"])?;
    number(&q["minimum_profile_revision"])?;
    let digest = text(&q["failure_model_sha256"])?;
    ensure(
        digest.len() == 64
            && digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "origin qualification digest",
    )?;
    let mut previous = None;
    for value in array(&v["required_backing_ids"])? {
        let current = uuid(value)?;
        ensure(
            previous.is_none_or(|p: &str| p < current),
            "origin required backing order",
        )?;
        previous = Some(current);
    }
    Ok(())
}

fn merge(into: &mut Requirements, values: Requirements) -> Result<()> {
    for (id, value) in values {
        if let Some(old) = into.insert(id, value.clone()) {
            ensure(old == value, "origin shared requirement identity")?;
        }
    }
    Ok(())
}

impl wire::ResourcePolicy for Origins {
    fn extra_features(&self) -> &'static [&'static str] {
        &["photara.resource-retention-evidence.v1"]
    }
    fn resources(&self, state: &Value, res: &wire::Resolver) -> Result<resources::Proof> {
        resources::verify_with_origin(
            &state["resource_state"],
            text(&state["root_id"])?,
            |r| res.get(r, 1),
            |source, context| match text(&source["origin"]["kind"])? {
                "authored" | "history" | "recovery" => ensure(
                    source["origin"]["source_id"] == context["root_id"],
                    "origin owning root identity",
                ),
                "explicit" => Ok(()),
                _ => Err("pending/unknown resource origin unsupported"),
            },
        )
    }

    fn evidence(
        &self,
        ctx: &wire::EvidenceContext<'_>,
        res: &wire::Resolver,
    ) -> Result<wire::Contribution> {
        evidence(ctx, res)
    }
}

fn evidence(ctx: &wire::EvidenceContext<'_>, res: &wire::Resolver) -> Result<wire::Contribution> {
    let mut out = wire::Contribution {
        logical: BTreeSet::new(),
        implementation: BTreeSet::new(),
    };
    let entries = walk(
        res,
        &ctx.root["retention_evidence"],
        Tree::Evidence,
        &mut out.implementation,
        &mut BTreeSet::new(),
        0,
    )?;
    let mut expected: BTreeMap<OriginKey, Requirements> = BTreeMap::new();
    let mut all_requirements = BTreeMap::new();
    for role in ctx.roles.values() {
        for source in role.associations.values() {
            let subset = requirements(res, &source["requirements"], &mut out)?;
            merge(&mut all_requirements, subset.clone())?;
            if source["origin"]["kind"] != "authored" {
                merge(
                    expected.entry(origin_key(&source["origin"])?).or_default(),
                    subset,
                )?;
            }
        }
    }
    let mut actual = BTreeSet::new();
    for entry in entries {
        let k = origin_key(&entry["key"])?;
        actual.insert(k.clone());
        let value = fetched(res, &entry["evidence"], &mut out.logical)?;
        evidence_schema(ctx, &value, &k)?;
        let subset = requirements(res, &value["requirements"], &mut out)?;
        ensure(!subset.is_empty(), "origin evidence nonempty subset")?;
        // Metadata identity consistency is global even when only recovery authority is checked.
        merge(&mut all_requirements, subset.clone())?;
        if ctx.recovery_only {
            if let Some(own) = expected.get(&k) {
                ensure(
                    own.iter().all(|(id, v)| subset.get(id) == Some(v)),
                    "origin recovery subset coverage",
                )?;
            }
        } else {
            ensure(
                expected.get(&k) == Some(&subset),
                "origin exact selected subset union",
            )?;
        }
        if k.0 == "explicit" {
            number(&value["revision"])?;
            continue;
        }
        context(
            ctx,
            res,
            &value,
            &k.0,
            !ctx.recovery_only || expected.contains_key(&k),
            &mut out,
        )?;
    }
    ensure(
        expected.keys().all(|k| actual.contains(k)),
        "origin unresolved evidence key",
    )?;
    if !ctx.recovery_only {
        ensure(
            actual == expected.keys().cloned().collect(),
            "origin unused evidence key",
        )?;
    }
    Ok(out)
}

fn evidence_schema(ctx: &wire::EvidenceContext<'_>, value: &Value, k: &OriginKey) -> Result<()> {
    let (id, extra) = match k.0.as_str() {
        "history" => (
            "photara.resource.history-retention-context",
            vec![
                "source_id",
                "state_root",
                "history",
                "promise",
                "requirements",
            ],
        ),
        "recovery" => (
            "photara.resource.recovery-retention-context",
            vec!["source_id", "state_root", "role", "requirements"],
        ),
        "explicit" => (
            "photara.resource.retention-policy",
            vec!["policy_id", "revision", "requirements"],
        ),
        _ => return Err("pending evidence unsupported"),
    };
    let mut extra_fields = vec!["library_id", "bootstrap_sha256"];
    extra_fields.extend(extra);
    schema(value, id, 1, &extra_fields)?;
    ensure(
        value["library_id"] == ctx.root["library_id"]
            && value["bootstrap_sha256"] == ctx.root["bootstrap_sha256"],
        "origin evidence package identity",
    )?;
    let identity = if k.0 == "explicit" {
        "policy_id"
    } else {
        "source_id"
    };
    ensure(
        uuid(&value[identity])? == k.1,
        "origin evidence key identity",
    )?;
    Ok(())
}

fn context(
    ctx: &wire::EvidenceContext<'_>,
    res: &wire::Resolver,
    value: &Value,
    kind: &str,
    require_verified: bool,
    out: &mut wire::Contribution,
) -> Result<()> {
    let selected: BTreeSet<Key> = std::iter::once(&ctx.root["active"])
        .chain(std::iter::once(&ctx.root["recovery"]))
        .chain(ctx.pins.iter().map(|p| &p["root"]))
        .map(key)
        .collect::<Result<_>>()?;
    let state_key = key(&value["state_root"])?;
    ensure(
        selected.contains(&state_key),
        "origin context selected root",
    )?;
    let placement = ctx
        .placements
        .get(&state_key)
        .ok_or("origin context placement")?;
    let state = fetched(res, &value["state_root"], &mut out.logical)?;
    schema(
        &state,
        "photara.package.state-root",
        2,
        &[
            "library_id",
            "bootstrap_sha256",
            "root_id",
            "authored_revision",
            "authored",
            "history",
            "resource_state",
            "operation_index",
            "accepted",
            "journal_inclusion",
            "predecessor",
            "inventory",
        ],
    )?;
    uuid(&state["root_id"])?;
    number(&state["authored_revision"])?;
    key(&state["history"])?;
    ensure(
        wire::reference(&state) == value["state_root"]
            && state["root_id"] == value["source_id"]
            && state["root_id"] == placement["root_id"]
            && state["library_id"] == ctx.root["library_id"]
            && state["bootstrap_sha256"] == ctx.root["bootstrap_sha256"],
        "origin context root identity",
    )?;
    if require_verified {
        ensure(
            ctx.states.get(&state_key) == Some(&state),
            "origin context verified state",
        )?;
    }
    if kind == "history" {
        ensure(
            value["promise"] == "reconstruct-selected-versions"
                && value["history"] == state["history"],
            "origin exact history promise",
        )?;
        fetched(res, &value["history"], &mut out.logical)?;
    } else {
        let role = &value["role"];
        match text(&role["kind"])? {
            "current-recovery" => {
                fields(role, &["kind"])?;
                ensure(
                    value["state_root"] == ctx.root["recovery"],
                    "origin current recovery role",
                )?;
            }
            "retained-recovery" => {
                fields(role, &["kind", "pin_id"])?;
                uuid(&role["pin_id"])?;
                ensure(
                    ctx.pins.iter().any(|p| {
                        p["pin_id"] == role["pin_id"]
                            && p["reason"] == "unresolved-recovery"
                            && p["root"] == value["state_root"]
                    }),
                    "origin retained recovery role",
                )?;
            }
            _ => return Err("origin recovery role variant"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn add(res: &mut wire::Resolver, value: Value) -> Value {
        let r = wire::reference(&value);
        res.objects.insert(key(&r).unwrap(), (value, 1));
        r
    }
    fn leaf(entries: &Value) -> Value {
        json!({"schema":{"id":"photara.resource.retention-evidence-leaf","version":1},"project_id":wire::PROJECT,"extensions":{},"count":entries.as_array().unwrap().len().to_string(),"entries":entries})
    }
    #[test]
    fn evidence_tree_checks_selected_order_and_exact_child_summary() {
        let mut res = wire::Resolver {
            objects: BTreeMap::new(),
            blobs: BTreeMap::new(),
            project: wire::PROJECT.into(),
            used: BTreeSet::new(),
        };
        let a = json!({"kind":"explicit","source_id":"71000000-0000-4000-8000-000000000001"});
        let b = json!({"kind":"history","source_id":"71000000-0000-4000-8000-000000000002"});
        let placeholder = wire::reference(&json!({}));
        let ar = add(&mut res, leaf(&json!([{"key":a,"evidence":placeholder}])));
        let br = add(&mut res, leaf(&json!([{"key":b,"evidence":placeholder}])));
        let mut branch = json!({"schema":{"id":"photara.resource.retention-evidence-branch","version":1},"project_id":wire::PROJECT,"extensions":{},"count":"2","children":[{"first":a,"last":a,"count":"1","child":ar},{"first":b,"last":b,"count":"1","child":br}]});
        let root = add(&mut res, branch.clone());
        assert_eq!(
            walk(
                &res,
                &root,
                Tree::Evidence,
                &mut BTreeSet::new(),
                &mut BTreeSet::new(),
                0
            )
            .unwrap()
            .len(),
            2
        );
        branch["children"][0]["last"] = b;
        let wrong = add(&mut res, branch);
        assert_eq!(
            walk(
                &res,
                &wrong,
                Tree::Evidence,
                &mut BTreeSet::new(),
                &mut BTreeSet::new(),
                0
            )
            .err(),
            Some("origin child summary")
        );
        let duplicate = add(
            &mut res,
            leaf(&json!([{"key":a,"evidence":placeholder},{"key":a,"evidence":placeholder}])),
        );
        assert_eq!(
            walk(
                &res,
                &duplicate,
                Tree::Evidence,
                &mut BTreeSet::new(),
                &mut BTreeSet::new(),
                0
            )
            .err(),
            Some("origin strict tree order")
        );
    }
    #[test]
    fn pending_is_not_a_synthetic_live_hold() {
        assert_eq!(
            origin_key(
                &json!({"kind":"pending","source_id":"71000000-0000-4000-8000-000000000001"})
            )
            .err(),
            Some("pending/unknown origin evidence unsupported")
        );
    }
}
