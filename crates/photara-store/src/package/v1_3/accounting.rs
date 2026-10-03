//! Frozen settled accounting metadata audit. No filesystem access, current
//! observation validation, capacity admission or writable authority. Nonempty
//! holds and retirement tickets require their selected-operation validator.
use super::TreeLimits;
use serde_json::Value;

/// Explicit runtime work budgets, never fixture-derived product limits.
#[derive(Clone, Copy, Debug)]
pub struct AccountingLimits {
    pub tree: TreeLimits,
    pub max_objects: usize,
    pub max_record_bytes: usize,
    pub max_tips: usize,
    pub max_path_components: usize,
}
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, &'static str>;
pub type ObjectKey = (String, u64);
pub struct AccountingMetadata {
    pub project_id: String,
    pub profile: String,
    pub incarnation: String,
    pub logical: BTreeSet<ObjectKey>,
    pub implementation: BTreeSet<ObjectKey>,
    pub tips: u64,
    pub sealed: u64,
    pub retained: u64,
    pub standing: u64,
    pub directory: u64,
    pub retained_directory: u64,
    pub total: u64,
    pub allocations: BTreeSet<String>,
    pub sealed_refs: BTreeMap<String, ObjectKey>,
    pub tip_ids: BTreeSet<String>,
    pub observations: BTreeMap<String, Value>,
}

fn ensure(ok: bool, why: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(why) }
}
fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let m = v.as_object().ok_or("accounting object")?;
    ensure(
        m.len() == names.len() && names.iter().all(|n| m.contains_key(*n)),
        "accounting exact fields",
    )
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or("accounting string")
}
fn number(v: &Value) -> Result<u64> {
    let s = text(v)?;
    let n = s.parse::<u64>().map_err(|_| "accounting decimal range")?;
    ensure(n.to_string() == s, "accounting canonical decimal")?;
    Ok(n)
}
fn array(v: &Value, limit: usize) -> Result<&Vec<Value>> {
    let a = v.as_array().ok_or("accounting array")?;
    ensure(a.len() <= limit, "accounting work limit")?;
    Ok(a)
}
fn id(v: &Value) -> Result<String> {
    let s = text(v)?;
    let u = uuid::Uuid::parse_str(s).map_err(|_| "accounting UUID")?;
    ensure(
        !u.is_nil() && u.to_string() == s,
        "accounting canonical nonnil UUID",
    )?;
    Ok(s.into())
}
fn digest(v: &Value) -> Result<String> {
    let s = text(v)?;
    ensure(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "accounting digest",
    )?;
    Ok(s.into())
}
fn canonical(v: &Value) -> Result<Vec<u8>> {
    photara_core::canonical_json(v).map_err(|_| "accounting canonical bytes")
}
fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
fn key(r: &Value) -> Result<ObjectKey> {
    fields(r, &["kind", "sha256", "byte_length"])?;
    ensure(
        r["kind"] == "json" && number(&r["byte_length"])? > 0,
        "accounting ObjectRef",
    )?;
    Ok((digest(&r["sha256"])?, number(&r["byte_length"])?))
}
fn object_key(v: &Value) -> Result<ObjectKey> {
    let b = canonical(v)?;
    Ok((
        hash(&b),
        u64::try_from(b.len()).map_err(|_| "accounting byte length")?,
    ))
}
fn path(v: &Value, limits: AccountingLimits) -> Result<Vec<String>> {
    let values = array(v, limits.max_path_components)?;
    ensure(
        !values.is_empty() && values.len() <= limits.max_path_components,
        "accounting work limit",
    )?;
    let p = values
        .iter()
        .map(|v| text(v).map(str::to_owned))
        .collect::<Result<Vec<_>>>()?;
    photara_core::contracts::resource::RelativeComponents::new(p.clone())
        .map_err(|_| "accounting portable path")?;
    Ok(p)
}
fn schema(v: &Value, name: &str, names: &[&str], project: &str) -> Result<()> {
    let mut all = vec!["schema", "project_id", "extensions"];
    all.extend_from_slice(names);
    fields(v, &all)?;
    ensure(
        v["schema"] == serde_json::json!({"id":name,"version":1}) && v["project_id"] == project,
        "accounting proposed schema/project dispatch",
    )?;
    ensure(
        v["extensions"].as_object().is_some_and(|m| {
            m.keys()
                .all(|k| photara_core::contracts::schema::QualifiedName::parse(k.clone()).is_ok())
        }),
        "accounting extension grammar",
    )
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or("accounting aggregate overflow")
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum Kind {
    Observation,
    Charge,
    Retained,
}
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
enum TreeKey {
    Id(String),
    Path(String, Vec<String>),
}
impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Observation => "photara.storage.local-observation",
            Self::Charge => "photara.storage.charge",
            Self::Retained => "photara.package.retained-file-charge",
        }
    }
    fn key(self, v: &Value, limits: AccountingLimits) -> Result<TreeKey> {
        if self == Self::Retained {
            fields(v, &["conversion_id", "path"])?;
            Ok(TreeKey::Path(
                id(&v["conversion_id"])?,
                path(&v["path"], limits)?,
            ))
        } else {
            Ok(TreeKey::Id(id(v)?))
        }
    }
    fn entry_key(self, v: &Value, limits: AccountingLimits) -> Result<TreeKey> {
        self.key(
            &v[match self {
                Self::Observation => "observation_id",
                Self::Charge => "allocation_id",
                Self::Retained => "key",
            }],
            limits,
        )
    }
    fn sum(self, v: &Value) -> Result<u64> {
        if self == Self::Observation {
            Ok(0)
        } else {
            number(
                &v[if self == Self::Retained {
                    "registered_charge"
                } else {
                    "charged_high_water"
                }],
            )
        }
    }
}
struct Tree {
    entries: Vec<Value>,
    first: Option<TreeKey>,
    last: Option<TreeKey>,
    sum: u64,
}
struct Reader<'a, F: FnMut(&Value) -> Result<Value>> {
    resolve: F,
    limits: AccountingLimits,
    project_id: &'a str,
    profile: &'a str,
    incarnation: &'a str,
    logical: BTreeSet<ObjectKey>,
    implementation: BTreeSet<ObjectKey>,
}
impl<F: FnMut(&Value) -> Result<Value>> Reader<'_, F> {
    fn get(&mut self, r: &Value, implementation: bool) -> Result<Value> {
        ensure(
            self.logical
                .len()
                .checked_add(self.implementation.len())
                .is_some_and(|n| n < self.limits.max_objects),
            "accounting work limit",
        )?;
        let expected = key(r)?;
        ensure(
            expected.1 <= self.limits.max_record_bytes as u64,
            "accounting work limit",
        )?;
        let v = (self.resolve)(r)?;
        ensure(
            object_key(&v)? == expected,
            "accounting resolved exact canonical bytes",
        )?;
        if implementation {
            self.implementation.insert(expected);
        } else {
            self.logical.insert(expected);
        }
        Ok(v)
    }
    fn tree(&mut self, r: &Value, kind: Kind) -> Result<Tree> {
        self.walk(r, kind, &mut BTreeSet::new(), 0)
    }
    #[allow(
        clippy::too_many_lines,
        reason = "One exact typed tree traversal validates child summaries and closure"
    )]
    fn walk(
        &mut self,
        r: &Value,
        kind: Kind,
        seen: &mut BTreeSet<ObjectKey>,
        depth: usize,
    ) -> Result<Tree> {
        ensure(
            depth < self.limits.tree.max_depth && seen.len() < self.limits.tree.max_pages,
            "accounting work limit",
        )?;
        ensure(seen.insert(key(r)?), "accounting unique tree traversal")?;
        let v = self.get(r, true)?;
        let leaf = v["schema"]["id"] == format!("{}-leaf", kind.prefix());
        let mut names = vec!["count", if leaf { "entries" } else { "children" }];
        if kind != Kind::Observation {
            names.push("charged_high_water");
        }
        schema(
            &v,
            &format!("{}-{}", kind.prefix(), if leaf { "leaf" } else { "branch" }),
            &names,
            self.project_id,
        )?;
        ensure(
            number(&v["count"])? <= self.limits.tree.max_entries as u64,
            "accounting work limit",
        )?;
        let mut entries = vec![];
        let mut total = 0;
        let mut first = None;
        let mut last = None;
        if leaf {
            let values = array(&v["entries"], self.limits.tree.max_leaf_entries)?;
            ensure(
                values.len() <= self.limits.tree.max_leaf_entries,
                "accounting work limit",
            )?;
            for entry in values {
                match kind {
                    Kind::Observation => {
                        fields(entry, &["observation_id", "observation"])?;
                        key(&entry["observation"])?;
                    }
                    Kind::Charge => {
                        fields(entry, &["allocation_id", "charge", "charged_high_water"])?;
                        key(&entry["charge"])?;
                    }
                    Kind::Retained => {
                        fields(
                            entry,
                            &[
                                "key",
                                "conversion_source",
                                "source_file",
                                "registered_charge",
                                "observation",
                            ],
                        )?;
                        key(&entry["conversion_source"])?;
                        key(&entry["observation"])?;
                        fields(&entry["source_file"], &["byte_length", "sha256"])?;
                        number(&entry["source_file"]["byte_length"])?;
                        digest(&entry["source_file"]["sha256"])?;
                    }
                }
                let current = kind.entry_key(entry, self.limits)?;
                ensure(
                    last.as_ref().is_none_or(|prior| prior < &current),
                    "accounting strict leaf order",
                )?;
                if first.is_none() {
                    first = Some(current.clone());
                }
                last = Some(current);
                total = add(total, kind.sum(entry)?)?;
                entries.push(entry.clone());
            }
        } else {
            let children = array(&v["children"], self.limits.tree.max_branch_children)?;
            ensure(
                children.len() >= 2 && children.len() <= self.limits.tree.max_branch_children,
                "accounting branch fanout",
            )?;
            for child in children {
                let mut names = vec!["first", "last", "count", "child"];
                if kind != Kind::Observation {
                    names.push("charged_high_water");
                }
                fields(child, &names)?;
                let lower = kind.key(&child["first"], self.limits)?;
                let upper = kind.key(&child["last"], self.limits)?;
                let count = number(&child["count"])?;
                ensure(
                    lower <= upper
                        && count > 0
                        && count <= self.limits.tree.max_entries as u64
                        && last.as_ref().is_none_or(|prior| prior < &lower),
                    "accounting strict child ranges",
                )?;
                let decoded = self.walk(&child["child"], kind, seen, depth + 1)?;
                ensure(
                    decoded.first.as_ref() == Some(&lower)
                        && decoded.last.as_ref() == Some(&upper)
                        && decoded.entries.len() as u64 == count,
                    "accounting exact child range/count",
                )?;
                if kind != Kind::Observation {
                    ensure(
                        decoded.sum == number(&child["charged_high_water"])?,
                        "accounting exact child charge",
                    )?;
                }
                if first.is_none() {
                    first = Some(lower);
                }
                last = Some(upper);
                total = add(total, decoded.sum)?;
                ensure(
                    entries
                        .len()
                        .checked_add(decoded.entries.len())
                        .is_some_and(|n| n <= self.limits.tree.max_entries),
                    "accounting work limit",
                )?;
                entries.extend(decoded.entries);
            }
        }
        ensure(
            entries.len() as u64 == number(&v["count"])?
                && entries.len() <= self.limits.tree.max_entries
                && (kind == Kind::Observation || total == number(&v["charged_high_water"])?),
            "accounting exact node aggregate",
        )?;
        Ok(Tree {
            entries,
            first,
            last,
            sum: total,
        })
    }
    fn observation(&mut self, r: &Value) -> Result<Value> {
        let v = self.get(r, false)?;
        self.observation_body(&v)?;
        Ok(v)
    }
    fn observation_body(&self, v: &Value) -> Result<()> {
        schema(
            v,
            "photara.storage.local-observation",
            &[
                "observation_id",
                "profile",
                "incarnation",
                "subject",
                "physical",
                "measured_extent",
                "charged_high_water",
            ],
            self.project_id,
        )?;
        id(&v["observation_id"])?;
        ensure(
            v["profile"] == self.profile && v["incarnation"] == self.incarnation,
            "accounting local observation binding",
        )?;
        fields(&v["physical"], &["device", "inode"])?;
        number(&v["physical"]["device"])?;
        ensure(
            number(&v["physical"]["inode"])? > 0,
            "accounting local inode",
        )?;
        ensure(
            number(&v["charged_high_water"])? >= number(&v["measured_extent"])?,
            "accounting registered extent lower bound",
        )?;
        let subject = &v["subject"];
        match text(&subject["kind"])? {
            "pack" | "whole-blob" => {
                fields(subject, &["kind", "allocation_id", "arena"])?;
                id(&subject["allocation_id"])?;
                ensure(
                    matches!(text(&subject["arena"])?, "data" | "metadata")
                        && (subject["kind"] != "whole-blob" || subject["arena"] == "data"),
                    "accounting allocation layout/arena",
                )?;
            }
            "retained-file" => {
                fields(subject, &["kind", "conversion_id", "namespace", "path"])?;
                id(&subject["conversion_id"])?;
                path(&subject["namespace"], self.limits)?;
                path(&subject["path"], self.limits)?;
            }
            _ => return Err("accounting local observation subject"),
        }
        Ok(())
    }
}
/// Read-only metadata audit. Embedded observations are checked for internal consistency,
/// never asserted to be fresh physical evidence. The caller authenticates selected controls.
/// # Errors
/// Refuses unsupported nonempty holds/tickets, incorrect typed closure, charge
/// aggregates, observation identities, retained-file manifests and work bounds.
pub fn audit_settled_accounting<P: super::LogicalRecords>(
    ledger: &Value,
    holds: &Value,
    provider: &P,
    limits: AccountingLimits,
) -> std::result::Result<AccountingMetadata, crate::package::PackageError> {
    let mut resolution_error = None;
    let resolve = |reference: &Value| {
        let object: crate::package::ObjectRef =
            serde_json::from_value(reference.clone()).map_err(|_| "accounting ObjectRef")?;
        // ConversionSource is semantic; accounting nodes/observations are ownership.
        // Select semantic membership only for the exact conversion Ref.
        let membership = if *reference == ledger["conversion_source"] {
            super::Membership::Semantic
        } else {
            super::Membership::Ownership
        };
        provider.resolve(&object, membership).map_err(|error| {
            resolution_error = Some(error);
            "accounting located object"
        })
    };
    validate(ledger, holds, resolve, limits).map_err(|why| {
        if let Some(error) = resolution_error {
            return error;
        }
        if why == "accounting work limit" {
            crate::package::PackageError::Limit
        } else {
            crate::package::PackageError::Integrity
        }
    })
}
#[allow(
    clippy::too_many_lines,
    reason = "One settled ledger audit checks exact observations and retained charge closure"
)]
fn validate<F: FnMut(&Value) -> Result<Value>>(
    ledger: &Value,
    holds: &Value,
    resolve: F,
    limits: AccountingLimits,
) -> Result<AccountingMetadata> {
    ensure(
        canonical(ledger)?.len() <= limits.max_record_bytes
            && canonical(holds)?.len() <= limits.max_record_bytes,
        "accounting work limit",
    )?;
    ensure(limits.max_objects >= 2, "accounting work limit")?;
    let project_id = id(&ledger["project_id"])?;
    let incarnation = id(&ledger["incarnation"])?;
    let profile = text(&ledger["profile"])?;
    photara_core::contracts::schema::QualifiedName::parse(profile.to_owned())
        .map_err(|_| "accounting profile grammar")?;
    schema(
        ledger,
        "photara.storage.ledger",
        &[
            "profile",
            "incarnation",
            "standing_control",
            "directory_allowance",
            "retained_directory_allowance",
            "tips",
            "sealed_charge_root",
            "observation_root",
            "retained_file_charge_root",
            "conversion_source",
            "retirement_tickets",
            "total_charge",
        ],
        &project_id,
    )?;

    schema(
        holds,
        "photara.storage.hold-leaf",
        &["count", "reserved", "consumed", "remaining", "entries"],
        &project_id,
    )?;
    ensure(
        holds["count"] == "0"
            && holds["reserved"] == "0"
            && holds["consumed"] == "0"
            && holds["remaining"] == "0"
            && holds["entries"] == serde_json::json!([]),
        "accounting nonempty holds unsupported in settled reader",
    )?;
    let mut reader = Reader {
        resolve,
        limits,
        project_id: &project_id,
        profile,
        incarnation: &incarnation,
        logical: BTreeSet::from([object_key(ledger)?]),
        implementation: BTreeSet::from([object_key(holds)?]),
    };
    let tickets = reader.get(&ledger["retirement_tickets"], true)?;
    schema(
        &tickets,
        "photara.storage.retirement-ticket-leaf",
        &["count", "charged_high_water", "entries"],
        &project_id,
    )?;
    ensure(
        tickets["count"] == "0"
            && tickets["charged_high_water"] == "0"
            && tickets["entries"] == serde_json::json!([]),
        "accounting nonempty retirement tickets unsupported in settled reader",
    )?;
    let observations = reader.tree(&ledger["observation_root"], Kind::Observation)?;
    let charges = reader.tree(&ledger["sealed_charge_root"], Kind::Charge)?;
    let retained = reader.tree(&ledger["retained_file_charge_root"], Kind::Retained)?;
    let mut by_ref = BTreeMap::new();
    let mut observation_ids = BTreeSet::new();
    for entry in observations.entries {
        let record = reader.observation(&entry["observation"])?;
        ensure(
            record["observation_id"] == entry["observation_id"]
                && observation_ids.insert(id(&record["observation_id"])?)
                && by_ref.insert(key(&entry["observation"])?, record).is_none(),
            "accounting exact unique observation membership",
        )?;
    }
    let mut used_observations = BTreeSet::new();
    let mut allocations = BTreeSet::new();
    let mut allocation_observations = BTreeMap::new();
    let mut tip_total = 0;
    let tips = array(&ledger["tips"], limits.max_tips)?;
    ensure(
        !tips.is_empty() && tips.len() <= limits.max_tips,
        "accounting tip count",
    )?;
    let mut prior_tip = None;
    for tip in tips {
        fields(
            tip,
            &[
                "allocation_id",
                "arena",
                "extent",
                "registered_charge",
                "observation",
            ],
        )?;
        let allocation = id(&tip["allocation_id"])?;
        ensure(
            prior_tip.as_ref().is_none_or(|prior| prior < &allocation),
            "accounting canonical tip order",
        )?;
        prior_tip = Some(allocation.clone());
        ensure(
            allocations.insert(allocation.clone()),
            "accounting duplicate registered allocation",
        )?;
        let extent = number(&tip["extent"])?;
        let registered = number(&tip["registered_charge"])?;
        // Current-tip observations are bounded ledger bodies, not packed dependencies
        // whose bytes would depend on their own future allocation measurement.
        let observation = &tip["observation"];
        reader.observation_body(observation)?;
        ensure(
            observation_ids.insert(id(&observation["observation_id"])?),
            "accounting disjoint unique tip observation",
        )?;
        ensure(
            observation["subject"]
                == serde_json::json!({"kind":"pack","allocation_id":allocation,"arena":tip["arena"]})
                && number(&observation["measured_extent"])? == extent
                && number(&observation["charged_high_water"])? == registered,
            "accounting exact tip attribution",
        )?;

        allocation_observations.insert(allocation.clone(), observation.clone());
        tip_total = add(tip_total, registered)?;
    }
    let tip_ids = allocations.clone();
    let mut sealed_refs = BTreeMap::new();
    for entry in &charges.entries {
        let value = reader.get(&entry["charge"], false)?;
        schema(
            &value,
            "photara.storage.sealed-charge",
            &[
                "allocation_id",
                "arena",
                "domain_incarnation",
                "measured_extent",
                "charged_high_water",
                "observation",
            ],
            &project_id,
        )?;
        let allocation = id(&value["allocation_id"])?;
        sealed_refs.insert(allocation.clone(), key(&entry["charge"])?);
        ensure(
            allocations.insert(allocation.clone()),
            "accounting sealed/tip identity double charge",
        )?;
        ensure(
            value["allocation_id"] == entry["allocation_id"]
                && value["charged_high_water"] == entry["charged_high_water"]
                && value["domain_incarnation"] == incarnation,
            "accounting exact sealed charge identity",
        )?;
        let observation_key = key(&value["observation"])?;
        let observation = by_ref
            .get(&observation_key)
            .ok_or("accounting selected sealed observation absent")?;
        ensure(
            observation["subject"]["allocation_id"] == allocation
                && matches!(
                    text(&observation["subject"]["kind"])?,
                    "pack" | "whole-blob"
                )
                && observation["subject"]["arena"] == value["arena"]
                && number(&observation["measured_extent"])? == number(&value["measured_extent"])?
                && number(&observation["charged_high_water"])?
                    == number(&value["charged_high_water"])?,
            "accounting exact sealed observation identity",
        )?;

        allocation_observations.insert(allocation.clone(), observation.clone());
        ensure(
            used_observations.insert(observation_key),
            "accounting duplicated sealed observation",
        )?;
    }

    let retained_total = retained.sum;
    if ledger["conversion_source"].is_null() {
        ensure(
            retained.entries.is_empty() && number(&ledger["retained_directory_allowance"])? == 0,
            "accounting absent conversion has obligations",
        )?;
    } else {
        let conversion = reader.get(&ledger["conversion_source"], false)?;

        schema(
            &conversion,
            "photara.package.conversion-source",
            &[
                "conversion_id",
                "snapshot_directory",
                "source_bootstrap_sha256",
                "source_commit_id",
                "source_format_version",
                "source_head_sha256",
                "files",
            ],
            &project_id,
        )?;
        id(&conversion["conversion_id"])?;
        id(&conversion["source_commit_id"])?;
        digest(&conversion["source_bootstrap_sha256"])?;
        digest(&conversion["source_head_sha256"])?;
        path(&conversion["snapshot_directory"], limits)?;
        fields(&conversion["source_format_version"], &["major", "minor"])?;
        ensure(
            conversion["source_format_version"]["major"]
                .as_u64()
                .is_some()
                && conversion["source_format_version"]["minor"]
                    .as_u64()
                    .is_some(),
            "accounting original format version",
        )?;
        let mut original_files = BTreeMap::new();
        let mut previous = None;
        for file in array(&conversion["files"], limits.tree.max_entries)? {
            fields(file, &["components", "byte_length", "sha256"])?;
            let p = path(&file["components"], limits)?;
            ensure(
                previous.as_ref().is_none_or(|old| old < &p)
                    && original_files.insert(p.clone(), file).is_none(),
                "accounting original source path order",
            )?;
            previous = Some(p);
            number(&file["byte_length"])?;
            digest(&file["sha256"])?;
        }
        ensure(
            retained.entries.len() == original_files.len(),
            "accounting exact retained file coverage",
        )?;

        let mut paths = BTreeSet::new();
        for entry in retained.entries {
            let p = path(&entry["key"]["path"], limits)?;
            ensure(
                paths.insert(p.clone()),
                "accounting duplicate retained path",
            )?;
            let original_file = original_files
                .get(&p)
                .ok_or("accounting unknown original retained path")?;
            ensure(
                entry["conversion_source"] == ledger["conversion_source"]
                    && entry["key"]["conversion_id"] == conversion["conversion_id"]
                    && entry["source_file"]
                        == serde_json::json!({"byte_length":original_file["byte_length"],"sha256":original_file["sha256"]}),
                "accounting original retained charge identity",
            )?;
            let observation_key = key(&entry["observation"])?;
            let observation = by_ref
                .get(&observation_key)
                .ok_or("accounting selected retained observation absent")?;
            ensure(
                observation["subject"]
                    == serde_json::json!({"kind":"retained-file","conversion_id":conversion["conversion_id"],"namespace":conversion["snapshot_directory"],"path":entry["key"]["path"]})
                    && number(&observation["measured_extent"])?
                        == number(&original_file["byte_length"])?
                    && number(&observation["charged_high_water"])?
                        == number(&entry["registered_charge"])?
                    && used_observations.insert(observation_key),
                "accounting exact retained local witness",
            )?;
        }
    }
    ensure(
        used_observations == by_ref.keys().cloned().collect(),
        "accounting exact observation use closure",
    )?;
    let standing = number(&ledger["standing_control"])?;
    let directory = number(&ledger["directory_allowance"])?;
    let retained_directory = number(&ledger["retained_directory_allowance"])?;

    let total = [
        tip_total,
        charges.sum,
        retained_total,
        standing,
        directory,
        retained_directory,
    ]
    .into_iter()
    .try_fold(0, add)?;
    ensure(
        total == number(&ledger["total_charge"])?,
        "accounting exact aggregate settlement",
    )?;
    Ok(AccountingMetadata {
        project_id: project_id.clone(),
        profile: profile.to_owned(),
        incarnation: incarnation.clone(),
        logical: reader.logical,
        implementation: reader.implementation,
        tips: tip_total,
        sealed: charges.sum,
        retained: retained_total,
        standing,
        directory,
        retained_directory,
        total,
        allocations,
        sealed_refs,
        tip_ids,
        observations: allocation_observations,
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::package::v1_3::{LogicalRecords, Membership};
    use crate::package::{ObjectRef, PackageError};
    pub(crate) struct Records(pub(crate) BTreeMap<ObjectKey, Value>);
    impl LogicalRecords for Records {
        fn resolve(
            &self,
            reference: &ObjectRef,
            membership: Membership,
        ) -> std::result::Result<Value, PackageError> {
            let value = self
                .0
                .get(&(
                    reference.sha256.as_str().to_owned(),
                    reference.byte_length.get(),
                ))
                .ok_or(PackageError::Integrity)?;
            let name = value["schema"]["id"].as_str().unwrap();
            let expected = if name.starts_with("photara.storage.")
                || name.starts_with("photara.package.retained-file-charge")
            {
                Membership::Ownership
            } else {
                Membership::Semantic
            };
            if membership != expected {
                return Err(PackageError::Integrity);
            }
            Ok(value.clone())
        }
    }
    pub(crate) fn setup() -> (Records, Value, Value, AccountingLimits) {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../../../docs/architecture/proposals/ps2/integrated/linked.json"
        ))
        .unwrap();
        let mut objects = BTreeMap::new();
        for row in corpus["records"].as_object().unwrap().values() {
            let value: Value = serde_json::from_str(row["canonical"].as_str().unwrap()).unwrap();
            objects.insert(object_key(&value).unwrap(), value);
        }
        for row in corpus["loose"].as_object().unwrap().values() {
            let value: Value = serde_json::from_str(row.as_str().unwrap()).unwrap();
            objects.insert(object_key(&value).unwrap(), value);
        }
        let ledger = objects
            .values()
            .find(|v| v["schema"]["id"] == "photara.storage.ledger")
            .unwrap()
            .clone();
        let holds = objects
            .values()
            .find(|v| v["schema"]["id"] == "photara.storage.hold-leaf")
            .unwrap()
            .clone();
        (
            Records(objects),
            ledger,
            holds,
            AccountingLimits {
                tree: TreeLimits {
                    max_depth: 16,
                    max_pages: 256,
                    max_entries: 4096,
                    max_leaf_entries: 3,
                    max_branch_children: 4,
                },
                max_objects: 4096,
                max_record_bytes: 1024 * 1024,
                max_tips: 8,
                max_path_components: 32,
            },
        )
    }
    #[test]
    fn frozen_settled_accounting_preserves_total_and_retained_closure() {
        let (provider, ledger, holds, limits) = setup();
        let proof = audit_settled_accounting(&ledger, &holds, &provider, limits).unwrap();
        assert_eq!(proof.total, 2_076_672);
        assert!(proof.retained > 0);
        assert_eq!(
            proof.allocations.len(),
            proof.tip_ids.len() + proof.sealed_refs.len()
        );
        assert_eq!(proof.observations.len(), proof.allocations.len());
        assert!(!proof.logical.is_empty());
        assert!(!proof.implementation.is_empty());
    }
    #[test]
    fn totals_incarnation_nonempty_holds_and_budget_changes_refuse() {
        let (provider, ledger, holds, limits) = setup();
        let mut altered = ledger.clone();
        altered["total_charge"] = serde_json::json!("1");
        assert!(audit_settled_accounting(&altered, &holds, &provider, limits).is_err());
        altered = ledger.clone();
        altered["incarnation"] = serde_json::json!("aaaaaaaa-0000-4000-8000-000000000001");
        assert!(audit_settled_accounting(&altered, &holds, &provider, limits).is_err());
        let mut changed_hold = holds.clone();
        changed_hold["count"] = serde_json::json!("1");
        assert!(audit_settled_accounting(&ledger, &changed_hold, &provider, limits).is_err());
        let mut small = limits;
        small.max_objects = 0;
        assert!(matches!(
            audit_settled_accounting(&ledger, &holds, &provider, small),
            Err(PackageError::Limit)
        ));
        small = limits;
        small.tree.max_entries = 0;
        assert!(matches!(
            audit_settled_accounting(&ledger, &holds, &provider, small),
            Err(PackageError::Limit)
        ));
    }
}
