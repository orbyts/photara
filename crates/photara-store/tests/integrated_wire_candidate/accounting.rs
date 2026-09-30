//! Settled proposed-coordinate accounting. No file I/O or runtime qualification.
//! Nonempty holds/tickets are an explicit unsupported transition boundary.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Result<T> = std::result::Result<T, &'static str>;
pub(super) type ObjectKey = (String, u64);
/// Original profile observations supplied by the caller, never authority from the ledger alone.
#[derive(Clone)]
pub(super) struct AllocationEvidence {
    pub(super) arena: String,
    pub(super) subject_kind: String,
    pub(super) extent: u64,
    pub(super) registered_charge: u64,
    pub(super) device: u64,
    pub(super) inode: u64,
}
#[derive(Clone)]
pub(super) struct FileEvidence {
    pub(super) bytes: Vec<u8>,
    pub(super) registered_charge: u64,
    pub(super) device: u64,
    pub(super) inode: u64,
}
/// Actual structural observation, independently matched to original registration by caller.
/// No content bytes and no claim that the current file digest was verified.
pub(super) struct StructuralFileEvidence {
    pub extent: u64,
    pub registered_charge: u64,
    pub device: u64,
    pub inode: u64,
    pub regular: bool,
}
pub(super) struct Evidence {
    pub(super) project_id: String,
    pub(super) profile: String,
    pub(super) incarnation: String,
    pub(super) allocations: BTreeMap<String, AllocationEvidence>,
    pub(super) retained_files: BTreeMap<Vec<String>, FileEvidence>,
    /// Exact original `ConversionSource` canonical bytes; empty only when no conversion is selected.
    pub(super) conversion: Option<Vec<u8>>,
    pub(super) standing_control: u64,
    pub(super) directory_allowance: u64,
    pub(super) retained_directory_allowance: u64,
}
pub(super) struct Proof {
    pub(super) logical: BTreeSet<ObjectKey>,
    pub(super) implementation: BTreeSet<ObjectKey>,
    pub(super) tips: u64,
    pub(super) sealed: u64,
    pub(super) retained: u64,
    pub(super) standing: u64,
    pub(super) directory: u64,
    pub(super) retained_directory: u64,
    pub(super) total: u64,
    pub(super) allocations: BTreeSet<String>,
    pub(super) sealed_refs: BTreeMap<String, ObjectKey>,
    pub(super) tip_ids: BTreeSet<String>,
    pub(super) observations: BTreeMap<String, Value>,
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
fn array(v: &Value) -> Result<&Vec<Value>> {
    let a = v.as_array().ok_or("accounting array")?;
    ensure(a.len() <= 4096, "accounting array bound")?;
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
fn path(v: &Value) -> Result<Vec<String>> {
    let values = array(v)?;
    ensure(
        !values.is_empty() && values.len() <= 32,
        "accounting path depth",
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
    fn key(self, v: &Value) -> Result<TreeKey> {
        if self == Self::Retained {
            fields(v, &["conversion_id", "path"])?;
            Ok(TreeKey::Path(id(&v["conversion_id"])?, path(&v["path"])?))
        } else {
            Ok(TreeKey::Id(id(v)?))
        }
    }
    fn entry_key(self, v: &Value) -> Result<TreeKey> {
        self.key(
            &v[match self {
                Self::Observation => "observation_id",
                Self::Charge => "allocation_id",
                Self::Retained => "key",
            }],
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
    project_id: &'a str,
    profile: &'a str,
    incarnation: &'a str,
    logical: BTreeSet<ObjectKey>,
    implementation: BTreeSet<ObjectKey>,
}
impl<F: FnMut(&Value) -> Result<Value>> Reader<'_, F> {
    fn get(&mut self, r: &Value, implementation: bool) -> Result<Value> {
        let expected = key(r)?;
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
            depth < 16 && seen.len() < 256 && seen.insert(key(r)?),
            "accounting bounded unique tree traversal",
        )?;
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
        let mut entries = vec![];
        let mut total = 0;
        let mut first = None;
        let mut last = None;
        if leaf {
            let values = array(&v["entries"])?;
            ensure(values.len() <= 3, "accounting leaf bound")?;
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
                let current = kind.entry_key(entry)?;
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
            let children = array(&v["children"])?;
            ensure(
                (2..=4).contains(&children.len()),
                "accounting branch fanout",
            )?;
            for child in children {
                let mut names = vec!["first", "last", "count", "child"];
                if kind != Kind::Observation {
                    names.push("charged_high_water");
                }
                fields(child, &names)?;
                let lower = kind.key(&child["first"])?;
                let upper = kind.key(&child["last"])?;
                let count = number(&child["count"])?;
                ensure(
                    lower <= upper
                        && count > 0
                        && count <= 4096
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
                entries.extend(decoded.entries);
            }
        }
        ensure(
            entries.len() as u64 == number(&v["count"])?
                && entries.len() <= 4096
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
                path(&subject["namespace"])?;
                path(&subject["path"])?;
            }
            _ => return Err("accounting local observation subject"),
        }
        Ok(())
    }
}
fn allocation_matches(observation: &Value, id: &str, e: &AllocationEvidence) -> Result<()> {
    ensure(
        observation["subject"]
            == serde_json::json!({"kind":e.subject_kind,"allocation_id":id,"arena":e.arena})
            && number(&observation["measured_extent"])? == e.extent
            && number(&observation["charged_high_water"])? == e.registered_charge
            && number(&observation["physical"]["device"])? == e.device
            && number(&observation["physical"]["inode"])? == e.inode,
        "accounting exact original allocation evidence",
    )
}
/// Validate settled accounting only. Caller authenticates ledger/hold selection through HEAD,
/// supplies original profile observations, and separately verifies physical/content closure.
#[allow(
    clippy::too_many_lines,
    reason = "Keep settled typed ledger and original evidence checks in authority order"
)]
pub(super) fn verify<F: FnMut(&Value) -> Result<Value>>(
    ledger: &Value,
    holds: &Value,
    resolve: F,
    evidence: &Evidence,
) -> Result<Proof> {
    validate(ledger, holds, resolve, Some(evidence), None)
}
/// Read-only retained source check. Strong default verification remains unchanged.
pub(super) fn verify_structural_source<F: FnMut(&Value) -> Result<Value>>(
    ledger: &Value,
    holds: &Value,
    resolve: F,
    evidence: &Evidence,
    files: &BTreeMap<Vec<String>, StructuralFileEvidence>,
) -> Result<Proof> {
    ensure(
        evidence.retained_files.is_empty(),
        "structural source cannot mix byte evidence",
    )?;
    validate(ledger, holds, resolve, Some(evidence), Some(files))
}
/// Read-only metadata proof. Embedded observations are checked for internal consistency,
/// never asserted to be fresh physical evidence. The caller authenticates selected controls.
pub(super) fn metadata<F: FnMut(&Value) -> Result<Value>>(
    ledger: &Value,
    holds: &Value,
    resolve: F,
) -> Result<Proof> {
    validate(ledger, holds, resolve, None, None)
}
#[allow(
    clippy::too_many_lines,
    reason = "Single typed parser shares metadata and independent evidence checks"
)]
fn validate<F: FnMut(&Value) -> Result<Value>>(
    ledger: &Value,
    holds: &Value,
    resolve: F,
    evidence: Option<&Evidence>,
    structural: Option<&BTreeMap<Vec<String>, StructuralFileEvidence>>,
) -> Result<Proof> {
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
    if let Some(e) = evidence {
        ensure(
            project_id == e.project_id && profile == e.profile && incarnation == e.incarnation,
            "accounting selected profile binding",
        )?;
    }
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
    let tips = array(&ledger["tips"])?;
    ensure(
        !tips.is_empty() && tips.len() <= 8,
        "accounting specimen tip bound",
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
        if let Some(e) = evidence {
            allocation_matches(
                observation,
                &allocation,
                e.allocations
                    .get(&allocation)
                    .ok_or("accounting original tip evidence absent")?,
            )?;
        }
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
        if let Some(e) = evidence {
            allocation_matches(
                observation,
                &allocation,
                e.allocations
                    .get(&allocation)
                    .ok_or("accounting original sealed evidence absent")?,
            )?;
        }
        allocation_observations.insert(allocation.clone(), observation.clone());
        ensure(
            used_observations.insert(observation_key),
            "accounting duplicated sealed observation",
        )?;
    }
    if let Some(e) = evidence {
        ensure(
            allocations == e.allocations.keys().cloned().collect(),
            "accounting exact observed allocation coverage",
        )?;
    }
    let retained_total = retained.sum;
    if ledger["conversion_source"].is_null() {
        ensure(
            retained.entries.is_empty() && number(&ledger["retained_directory_allowance"])? == 0,
            "accounting absent conversion has obligations",
        )?;
        if let Some(e) = evidence {
            ensure(
                e.conversion.is_none()
                    && e.retained_files.is_empty()
                    && structural.is_none_or(BTreeMap::is_empty),
                "accounting absent conversion evidence",
            )?;
        }
    } else {
        let conversion = reader.get(&ledger["conversion_source"], false)?;
        if let Some(e) = evidence {
            ensure(
                e.conversion.as_ref() == Some(&canonical(&conversion)?),
                "accounting unchanged original ConversionSource bytes",
            )?;
        }
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
        path(&conversion["snapshot_directory"])?;
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
        for file in array(&conversion["files"])? {
            fields(file, &["components", "byte_length", "sha256"])?;
            let p = path(&file["components"])?;
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
        if let Some(e) = evidence {
            ensure(
                structural.map_or(e.retained_files.len(), BTreeMap::len) == original_files.len(),
                "accounting exact original file coverage",
            )?;
        }
        let mut paths = BTreeSet::new();
        for entry in retained.entries {
            let p = path(&entry["key"]["path"])?;
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
            if let Some(files) = structural {
                let physical = files
                    .get(&p)
                    .ok_or("accounting original retained structural witness absent")?;
                ensure(
                    physical.regular
                        && physical.inode > 0
                        && physical.extent == number(&original_file["byte_length"])?
                        && number(&entry["registered_charge"])? == physical.registered_charge
                        && number(&observation["physical"]["device"])? == physical.device
                        && number(&observation["physical"]["inode"])? == physical.inode,
                    "accounting exact retained structural extent and witness",
                )?;
            } else if let Some(e) = evidence {
                let physical = e
                    .retained_files
                    .get(&p)
                    .ok_or("accounting original retained witness absent")?;
                ensure(
                    physical.bytes.len() as u64 == number(&original_file["byte_length"])?
                        && hash(&physical.bytes) == original_file["sha256"]
                        && number(&entry["registered_charge"])? == physical.registered_charge
                        && number(&observation["physical"]["device"])? == physical.device
                        && number(&observation["physical"]["inode"])? == physical.inode,
                    "accounting exact original retained bytes and witness",
                )?;
            }
        }
    }
    ensure(
        used_observations == by_ref.keys().cloned().collect(),
        "accounting exact observation use closure",
    )?;
    let standing = number(&ledger["standing_control"])?;
    let directory = number(&ledger["directory_allowance"])?;
    let retained_directory = number(&ledger["retained_directory_allowance"])?;
    if let Some(e) = evidence {
        ensure(
            standing == e.standing_control
                && directory == e.directory_allowance
                && retained_directory == e.retained_directory_allowance,
            "accounting original namespace/control allowances",
        )?;
    }
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
    Ok(Proof {
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
mod tests {
    use super::*;
    use serde_json::json;
    fn unhex(s: &str) -> Vec<u8> {
        s.as_bytes()
            .chunks_exact(2)
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect()
    }
    fn rounded(n: usize) -> u64 {
        u64::try_from(n.div_ceil(4096) * 4096).unwrap()
    }
    fn fixture() -> (Value, Value, BTreeMap<ObjectKey, Value>, Evidence) {
        let c: Value = serde_json::from_str(include_str!(
            "../../../../docs/architecture/proposals/ps2/integrated/linked.json"
        ))
        .unwrap();
        let mut objects: BTreeMap<_, _> = c["records"]
            .as_object()
            .unwrap()
            .values()
            .map(|r| {
                let v: Value = serde_json::from_str(r["canonical"].as_str().unwrap()).unwrap();
                (object_key(&v).unwrap(), v)
            })
            .collect();
        for raw in c["loose"].as_object().unwrap().values() {
            let v: Value = serde_json::from_str(raw.as_str().unwrap()).unwrap();
            objects.insert(object_key(&v).unwrap(), v);
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
        let conversion = objects
            .values()
            .find(|v| v["schema"]["id"] == "photara.package.conversion-source")
            .unwrap();
        let allocations = c["allocations"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(id, v)| {
                let bytes = unhex(v["hex"].as_str().unwrap());
                (
                    id.clone(),
                    AllocationEvidence {
                        arena: v["arena"].as_str().unwrap().into(),
                        subject_kind: "pack".into(),
                        extent: bytes.len() as u64,
                        registered_charge: rounded(bytes.len()),
                        device: number(&v["witness"]["device"]).unwrap(),
                        inode: number(&v["witness"]["inode"]).unwrap(),
                    },
                )
            })
            .collect();
        let retained_files = c["source_files"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, value)| {
                let bytes = unhex(value.as_str().unwrap());
                let witness = &c["source_witnesses"][name];
                (
                    name.split('/').map(str::to_owned).collect(),
                    FileEvidence {
                        registered_charge: rounded(bytes.len()),
                        bytes,
                        device: number(&witness["device"]).unwrap(),
                        inode: number(&witness["inode"]).unwrap(),
                    },
                )
            })
            .collect();
        let evidence = Evidence {
            project_id: "10000000-0000-4000-8000-000000000001".into(),
            profile: "example.ps2.synthetic-local-profile-v1".into(),
            incarnation: "96000000-0000-4000-8000-000000000001".into(),
            allocations,
            retained_files,
            conversion: Some(canonical(conversion).unwrap()),
            standing_control: 131_072,
            directory_allowance: 16_384,
            retained_directory_allowance: 16_384,
        };
        (ledger, holds, objects, evidence)
    }
    fn check(
        ledger: &Value,
        holds: &Value,
        objects: &BTreeMap<ObjectKey, Value>,
        evidence: &Evidence,
    ) -> Result<Proof> {
        verify(
            ledger,
            holds,
            |r| {
                objects
                    .get(&key(r)?)
                    .cloned()
                    .ok_or("test missing accounting object")
            },
            evidence,
        )
    }
    #[test]
    fn integrated_accounting_actual_settled_roots_and_breakdown() -> Result<()> {
        let (l, h, objects, e) = fixture();
        let p = check(&l, &h, &objects, &e)?;
        ensure(
            p.tips == 7 * 262_144
                && p.sealed == 4096
                && p.retained == 18 * 4096
                && p.standing == 131_072
                && p.directory == 16_384
                && p.retained_directory == 16_384
                && p.total == 2_076_672
                && p.allocations.len() == 8,
            "settled exact measured breakdown",
        )?;
        ensure(
            p.logical.contains(&object_key(&l)?)
                && p.implementation.contains(&object_key(&h)?)
                && p.logical.is_disjoint(&p.implementation),
            "separate complete accounting sets",
        )
    }
    #[test]
    fn integrated_accounting_original_witness_and_charge_basis_cannot_rebind() -> Result<()> {
        for mode in ["inode", "charge", "profile", "retained", "allowance"] {
            let (l, h, objects, mut e) = fixture();
            match mode {
                "inode" => e.allocations.values_mut().next().unwrap().inode += 1,
                "charge" => e.allocations.values_mut().next().unwrap().registered_charge += 4096,
                "profile" => e.incarnation = "96000000-0000-4000-8000-000000000002".into(),
                "retained" => e.retained_files.values_mut().next().unwrap().bytes[0] ^= 1,
                _ => e.directory_allowance = 0,
            }
            ensure(
                check(&l, &h, &objects, &e).is_err(),
                "original physical/evidence mismatch accepted",
            )?;
        }
        Ok(())
    }
    #[test]
    fn integrated_accounting_nonempty_transition_or_unknown_fields_fence() -> Result<()> {
        let (l, h, objects, e) = fixture();
        for mode in ["hold", "ticket", "ledger", "tip", "tree"] {
            let (mut ledger, mut holds, mut objects) = (l.clone(), h.clone(), objects.clone());
            match mode {
                "hold" => holds["remaining"] = json!("1"),
                "ticket" => {
                    let mut ticket = objects[&key(&ledger["retirement_tickets"])?].clone();
                    ticket["entries"] = json!([{}]);
                    let k = object_key(&ticket)?;
                    ledger["retirement_tickets"] =
                        json!({"kind":"json","sha256":k.0,"byte_length":k.1.to_string()});
                    objects.insert(k, ticket);
                }
                "ledger" => ledger["unknown_required"] = json!(true),
                "tip" => ledger["tips"][0]["registered_charge"] = json!("0"),
                _ => {
                    let mut tree = objects[&key(&ledger["observation_root"])?].clone();
                    tree["schema"]["version"] = json!(2);
                    let k = object_key(&tree)?;
                    ledger["observation_root"] =
                        json!({"kind":"json","sha256":k.0,"byte_length":k.1.to_string()});
                    objects.insert(k, tree);
                }
            }
            ensure(
                check(&ledger, &holds, &objects, &e).is_err(),
                "unproved transition/layout accepted",
            )?;
        }
        Ok(())
    }
    #[test]
    fn integrated_accounting_metadata_is_exact_but_not_physical_evidence() -> Result<()> {
        let (l, h, objects, e) = fixture();
        let read = |ledger: &Value| {
            metadata(ledger, &h, |r| {
                objects
                    .get(&key(r)?)
                    .cloned()
                    .ok_or("missing metadata object")
            })
        };
        let full = check(&l, &h, &objects, &e)?;
        let parsed = read(&l)?;
        ensure(
            parsed.logical == full.logical
                && parsed.implementation == full.implementation
                && parsed.total == full.total
                && parsed.sealed_refs == full.sealed_refs
                && parsed.tip_ids == full.tip_ids
                && parsed.observations == full.observations,
            "metadata and physical parsers diverged",
        )?;
        let mut substituted = l.clone();
        substituted["tips"][0]["observation"]["physical"]["inode"] = json!("999999");
        read(&substituted)?;
        ensure(
            check(&substituted, &h, &objects, &e).is_err(),
            "metadata substituted witness conferred physical authority",
        )?;
        let mut undercharged = l.clone();
        undercharged["tips"][0]["registered_charge"] = json!("0");
        undercharged["tips"][0]["observation"]["charged_high_water"] = json!("0");
        undercharged["total_charge"] = json!((full.total - full.tips / 7).to_string());
        ensure(
            matches!(
                read(&undercharged),
                Err("accounting registered extent lower bound")
            ),
            "coherent aggregate undercharge accepted",
        )?;
        Ok(())
    }
    #[test]
    fn integrated_accounting_tip_bodies_are_ordered_and_disjoint() -> Result<()> {
        let (l, h, objects, e) = fixture();
        let mut reordered = l.clone();
        reordered["tips"].as_array_mut().unwrap().swap(0, 1);
        ensure(
            matches!(
                check(&reordered, &h, &objects, &e),
                Err("accounting canonical tip order")
            ),
            "unordered tips accepted",
        )?;
        let packed = objects
            .values()
            .find(|v| {
                v["schema"]["id"] == "photara.storage.local-observation"
                    && v["subject"]["kind"] == "pack"
            })
            .ok_or("missing packed sealed observation")?;
        let mut duplicate = l.clone();
        duplicate["tips"][0]["observation"]["observation_id"] = packed["observation_id"].clone();
        ensure(
            matches!(
                check(&duplicate, &h, &objects, &e),
                Err("accounting disjoint unique tip observation")
            ),
            "tip reused packed observation identity",
        )?;
        let mut indirect = l.clone();
        let k = object_key(&l["tips"][0]["observation"])?;
        indirect["tips"][0]["observation"] =
            json!({"kind":"json","sha256":k.0,"byte_length":k.1.to_string()});
        ensure(
            check(&indirect, &h, &objects, &e).is_err(),
            "indirect measured tip accepted",
        )
    }
    #[test]
    fn integrated_accounting_resolver_must_return_exact_original_canonical_bytes() -> Result<()> {
        let (l, h, objects, e) = fixture();
        ensure(
            verify(
                &l,
                &h,
                |r| {
                    let mut value = objects.get(&key(r)?).cloned().ok_or("missing")?;
                    value["extensions"] = json!({"example.substitution":true});
                    Ok(value)
                },
                &e,
            )
            .is_err(),
            "resolver silently substituted accounting bytes",
        )
    }
}
