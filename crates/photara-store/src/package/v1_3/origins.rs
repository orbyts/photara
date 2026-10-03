//! Selected retention authority, separate from resource metadata/qualification.
//! Pending origins fail closed until a production original-operation verifier exists.
use super::super::{ObjectRef, PackageError};
use super::{LogicalRecords, Membership, ResourceLimits, ResourceMetadata, SelectionIdentity};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
type Key = ObjectRef;
type Result<T> = std::result::Result<T, &'static str>;
type OriginKey = (String, String);
type Requirements = BTreeMap<String, Value>;
/// Inputs are constructed only after the selected role's complete semantic proof.
pub(crate) struct OriginRole<'a> {
    pub state: &'a Value,
    pub resources: Option<&'a ResourceMetadata>,
}
struct Context<'a> {
    root: &'a Value,
    pins: &'a [Value],
    roles: &'a BTreeMap<String, OriginRole<'a>>,
    states: BTreeMap<Key, Value>,
    recovery_only: bool,
}
#[derive(Default)]
struct Contribution {
    logical: BTreeSet<Key>,
    implementation: BTreeSet<Key>,
}
struct Resolver<'a> {
    provider: &'a dyn LogicalRecords,
    identity: &'a SelectionIdentity,
    limits: ResourceLimits,
    seen: std::cell::RefCell<BTreeSet<Key>>,
    error: std::cell::RefCell<Option<PackageError>>,
}
impl Resolver<'_> {
    fn get(&self, r: &Value, _tag: u8) -> Result<Value> {
        let r = key(r)?;
        ensure(
            r.byte_length.get() <= self.limits.max_record_bytes as u64,
            "origin work limit",
        )?;
        let mut seen = self.seen.borrow_mut();
        ensure(
            seen.contains(&r) || seen.len() < self.limits.max_objects,
            "origin work limit",
        )?;
        seen.insert(r.clone());
        self.provider
            .resolve(&r, Membership::Semantic)
            .map_err(|e| {
                *self.error.borrow_mut() = Some(e);
                "origin resolution"
            })
    }
    fn schema(&self, v: &Value, id: &str, version: u64, fields: &[&str]) -> Result<()> {
        super::selected::schema(v, id, version, fields, self.identity.project).map_err(|e| {
            *self.error.borrow_mut() = Some(e);
            "origin schema"
        })
    }
}
fn ensure(ok: bool, why: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(why) }
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or("origin string")
}
fn number(v: &Value) -> Result<u64> {
    let s = text(v)?;
    let n = s.parse::<u64>().map_err(|_| "origin decimal")?;
    ensure(n.to_string() == s, "origin canonical decimal")?;
    Ok(n)
}
fn fields(v: &Value, keys: &[&str]) -> Result<()> {
    super::resources::fields(v, keys)
}
fn key(v: &Value) -> Result<Key> {
    super::selected::json_ref(v).map_err(|_| "origin reference")
}
fn reference(v: &Value) -> Result<Key> {
    let bytes = photara_core::canonical_json(v).map_err(|_| "origin canonical")?;
    Ok(ObjectRef {
        kind: super::super::ObjectKind::Json,
        sha256: super::super::digest(&bytes),
        byte_length: super::super::DecimalU64::parse(&bytes.len().to_string())
            .map_err(|_| "origin length")?,
    })
}
/// Full mode proves the exact evidence union; recovery mode proves only its own
/// subset plus selected metadata consistency. Neither establishes writable custody.
#[allow(
    clippy::too_many_arguments,
    reason = "Explicit selected role, identity and work-budget inputs"
)]
pub(crate) fn inspect<'a>(
    provider: &impl LogicalRecords,
    root: &'a Value,
    pins: &'a [Value],
    roles: &'a BTreeMap<String, OriginRole<'a>>,
    identity: &SelectionIdentity,
    limits: ResourceLimits,
    recovery_only: bool,
) -> std::result::Result<BTreeSet<ObjectRef>, PackageError> {
    let res = Resolver {
        provider,
        identity,
        limits,
        seen: std::cell::RefCell::default(),
        error: std::cell::RefCell::default(),
    };
    let checked = || -> Result<Contribution> {
        ensure(
            root["project_id"] == identity.project.to_string()
                && root["library_id"] == identity.library.to_string()
                && root["bootstrap_sha256"] == identity.bootstrap_sha256.as_str(),
            "origin selected identity",
        )?;
        let states = roles
            .values()
            .map(|r| Ok((reference(r.state)?, r.state.clone())))
            .collect::<Result<_>>()?;
        evidence(
            &Context {
                root,
                pins,
                roles,
                states,
                recovery_only,
            },
            &res,
        )
    };
    let result = checked().map_err(|why| {
        res.error
            .borrow_mut()
            .take()
            .unwrap_or(if why == "origin work limit" {
                PackageError::Limit
            } else {
                PackageError::Integrity
            })
    })?;
    Ok(result
        .logical
        .union(&result.implementation)
        .cloned()
        .collect())
}
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
    Ok(a)
}
fn fetched(res: &Resolver<'_>, r: &Value, set: &mut BTreeSet<Key>) -> Result<Value> {
    let v = res.get(r, 1)?;
    ensure(
        reference(&v)? == key(r)?,
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
    res: &Resolver<'_>,
    r: &Value,
    kind: Tree,
    nodes: &mut BTreeSet<Key>,
    visited: &mut BTreeSet<Key>,
    depth: usize,
) -> Result<Vec<Value>> {
    ensure(
        depth < res.limits.tree.max_depth
            && visited.len() < res.limits.tree.max_pages
            && visited.insert(key(r)?),
        "origin work limit",
    )?;
    let v = fetched(res, r, nodes)?;
    let leaf = v["schema"]["id"] == kind.node(true);
    let mut extras = vec!["count", if leaf { "entries" } else { "children" }];
    if matches!(kind, Tree::Requirements) {
        extras.push("selection_kind");
    }
    res.schema(&v, kind.node(leaf), 1, &extras)?;
    if matches!(kind, Tree::Requirements) {
        ensure(
            v["selection_kind"] == "requirements",
            "origin requirement tree kind",
        )?;
    }
    let mut entries = Vec::new();
    if leaf {
        let es = array(&v["entries"])?;
        ensure(
            es.len() <= res.limits.tree.max_leaf_entries && es.len() <= res.limits.tree.max_entries,
            "origin work limit",
        )?;
        entries.clone_from(es);
    } else {
        let children = array(&v["children"])?;
        ensure(
            children.len() >= 2 && children.len() <= res.limits.tree.max_branch_children,
            "origin branch fanout",
        )?;
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
            ensure(
                entries.len() <= res.limits.tree.max_entries,
                "origin work limit",
            )?;
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
    res: &Resolver<'_>,
    r: &Value,
    contribution: &mut Contribution,
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
        res.schema(
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

#[allow(
    clippy::too_many_lines,
    reason = "Exact selected origin union and optional original-operation proof in one pass"
)]
fn evidence(ctx: &Context<'_>, res: &Resolver<'_>) -> Result<Contribution> {
    let mut out = Contribution {
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
        let Some(resources) = role.resources else {
            continue;
        };
        for source in resources.associations.values() {
            let kind = text(&source["origin"]["kind"])?;
            ensure(
                !["authored", "history", "recovery"].contains(&kind)
                    || source["origin"]["source_id"] == role.state["root_id"],
                "origin owning root identity",
            )?;
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
        evidence_schema(ctx, res, &value, &k)?;
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

fn evidence_schema(
    ctx: &Context<'_>,
    res: &Resolver<'_>,
    value: &Value,
    k: &OriginKey,
) -> Result<()> {
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
    res.schema(value, id, 1, &extra_fields)?;
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
    ctx: &Context<'_>,
    res: &Resolver<'_>,
    value: &Value,
    kind: &str,
    require_verified: bool,
    out: &mut Contribution,
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
    let state = fetched(res, &value["state_root"], &mut out.logical)?;
    res.schema(
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
        reference(&state)? == key(&value["state_root"])?
            && state["root_id"] == value["source_id"]
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
    const PROJECT: &str = "71000000-0000-4000-8000-000000000001";
    const LIBRARY: &str = "71000000-0000-4000-8000-000000000002";
    const POLICY: &str = "71000000-0000-4000-8000-000000000003";
    const ROOT: &str = "71000000-0000-4000-8000-000000000004";
    const REQUIREMENT: &str = "71000000-0000-4000-8000-000000000005";
    const SECOND: &str = "71000000-0000-4000-8000-000000000006";
    #[derive(Default)]
    struct Records(BTreeMap<ObjectRef, Value>);
    impl LogicalRecords for Records {
        fn resolve(
            &self,
            object: &ObjectRef,
            membership: Membership,
        ) -> std::result::Result<Value, PackageError> {
            assert_eq!(membership, Membership::Semantic);
            self.0.get(object).cloned().ok_or(PackageError::Integrity)
        }
    }
    fn put(records: &mut Records, value: Value) -> Value {
        let r = reference(&value).unwrap();
        records.0.insert(r.clone(), value);
        serde_json::to_value(r).unwrap()
    }
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Owned fixture JSON assembled at call sites"
    )]
    fn record(id: &str, body: Value) -> Value {
        let mut v = json!({"schema":{"id":id,"version":1},"project_id":PROJECT,"extensions":{}});
        v.as_object_mut()
            .unwrap()
            .extend(body.as_object().unwrap().clone());
        v
    }
    fn limits() -> ResourceLimits {
        ResourceLimits {
            tree: super::super::TreeLimits {
                max_depth: 16,
                max_pages: 128,
                max_entries: 256,
                max_leaf_entries: 4,
                max_branch_children: 4,
            },
            max_objects: 256,
            max_record_bytes: 65536,
        }
    }
    fn requirement(id: &str) -> Value {
        record(
            "photara.resource.retention-requirement",
            json!({"requirement_id":id,"revision":"1","resource_id":PROJECT,"version_id":LIBRARY,"minimum_qualified_copies":"1","required_backing_ids":[],"required_qualification":{"profile_id":PROJECT,"minimum_profile_revision":"1","failure_model_sha256":"a".repeat(64)}}),
        )
    }
    fn selection(records: &mut Records, ids: &[&str]) -> Value {
        let entries = ids
            .iter()
            .map(|id| json!({"id":id,"record":put(records,requirement(id))}))
            .collect::<Vec<_>>();
        put(
            records,
            record(
                "photara.resource.selection-leaf",
                json!({"selection_kind":"requirements","count":entries.len().to_string(),"entries":entries}),
            ),
        )
    }
    #[test]
    fn exact_policy_union_and_recovery_subset_are_distinct() {
        let mut records = Records::default();
        let own = selection(&mut records, &[REQUIREMENT]);
        let union = selection(&mut records, &[REQUIREMENT, SECOND]);
        let policy = put(
            &mut records,
            record(
                "photara.resource.retention-policy",
                json!({"library_id":LIBRARY,"bootstrap_sha256":"a".repeat(64),"policy_id":POLICY,"revision":"1","requirements":union}),
            ),
        );
        let tree = put(
            &mut records,
            record(
                "photara.resource.retention-evidence-leaf",
                json!({"count":"1","entries":[{"key":{"kind":"explicit","source_id":POLICY},"evidence":policy}]}),
            ),
        );
        let state = json!({"root_id":ROOT});
        let metadata = ResourceMetadata {
            closure: BTreeSet::new(),
            requirements: BTreeMap::new(),
            associations: BTreeMap::from([(
                ROOT.into(),
                json!({"origin":{"kind":"explicit","source_id":POLICY},"requirements":own}),
            )]),
        };
        let roles = BTreeMap::from([(
            "recovery".into(),
            OriginRole {
                state: &state,
                resources: Some(&metadata),
            },
        )]);
        let root = json!({"project_id":PROJECT,"library_id":LIBRARY,"bootstrap_sha256":"a".repeat(64),"retention_evidence":tree});
        let identity = SelectionIdentity {
            project: super::super::super::PackageUuid::parse(PROJECT).unwrap(),
            library: super::super::super::PackageUuid::parse(LIBRARY).unwrap(),
            bootstrap_sha256: super::super::super::Sha256Hex::parse(&"a".repeat(64)).unwrap(),
        };
        assert!(inspect(&records, &root, &[], &roles, &identity, limits(), true).is_ok());
        assert!(inspect(&records, &root, &[], &roles, &identity, limits(), false).is_err());
        let mut bounded = limits();
        bounded.max_objects = 1;
        assert!(matches!(
            inspect(&records, &root, &[], &roles, &identity, bounded, true),
            Err(PackageError::Limit)
        ));
        let mut changed = records.0[&key(&policy).unwrap()].clone();
        changed["requirements"] = selection(&mut records, &[REQUIREMENT]);
        let policy = put(&mut records, changed);
        let tree = put(
            &mut records,
            record(
                "photara.resource.retention-evidence-leaf",
                json!({"count":"1","entries":[{"key":{"kind":"explicit","source_id":POLICY},"evidence":policy}]}),
            ),
        );
        let mut root = root;
        root["retention_evidence"] = tree;
        assert!(inspect(&records, &root, &[], &roles, &identity, limits(), false).is_ok());
        records.0.remove(&key(&policy).unwrap());
        assert!(inspect(&records, &root, &[], &roles, &identity, limits(), false).is_err());
    }
    #[test]
    fn pending_and_conflicting_requirement_identity_cannot_supply_authority() {
        assert!(origin_key(&json!({"kind":"pending","source_id":ROOT})).is_err());
        let mut merged = BTreeMap::from([(REQUIREMENT.into(), requirement(REQUIREMENT))]);
        let mut changed = requirement(REQUIREMENT);
        changed["minimum_qualified_copies"] = json!("2");
        assert!(merge(&mut merged, BTreeMap::from([(REQUIREMENT.into(), changed)])).is_err());
    }
}
