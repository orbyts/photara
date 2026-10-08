//! One registered disposable activation slot. No Library database or path admission.
use super::{
    DIRECTORY, File, FlockOperation, IoResult, MetadataExt, Mode, OFlags, Path, PermissionsExt,
    Scope, Value, Write, barrier, canonical, flock, hash, invalid, json, number, openat, package,
    pin, read, regular, renameat, session_host,
};
use crate::package::v1_3::activation::{self, Snapshot};

const SNAPSHOT: &str = "activation.json";
struct Storage {
    scope: Scope,
    directory: File,
    lock: File,
    registration: Value,
    registration_bytes: Vec<u8>,
    bindings: Value,
    current: Snapshot,
    limits: activation::Limits,
    faults: std::collections::BTreeSet<String>,
    uncertain_candidate: Option<Vec<u8>>,
}
fn bad(e: impl std::fmt::Debug) -> std::io::Error {
    std::io::Error::other(format!("activation refused: {e:?}"))
}
fn record(id: &str, kind: &str, body: Value) -> Value {
    let mut value = json!({"id":id,"kind":kind,"version":1,"body":null});
    value["body"] = body;
    value
}
fn rid(id: &str, role: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("photara disposable activation {id} {role}").as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 15) | 64;
    bytes[8] = (bytes[8] & 63) | 128;
    uuid::Uuid::from_bytes(bytes).to_string()
}
fn add(body: &mut Value, value: Value) -> IoResult<Value> {
    let reference = activation::record_ref(&value).map_err(bad)?;
    let rows = body["records"].as_array_mut().ok_or_else(invalid)?;
    if let Some(old) = rows.iter().find(|v| v["id"] == value["id"]) {
        if old != &value {
            return Err(invalid());
        }
    } else {
        rows.push(value);
        rows.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    }
    Ok(reference)
}
macro_rules! put {
    ($body:expr,$value:expr) => {{
        let value = $value;
        add($body, value)
    }};
}
impl Storage {
    fn original_candidate(&self, id: &str) -> IoResult<Option<Snapshot>> {
        let mut result: Option<Snapshot> = None;
        for (count, entry) in rustix::fs::Dir::read_from(&self.directory)?.enumerate() {
            if count > self.limits.max_records + 5 {
                return Err(invalid());
            }
            let entry = entry?;
            let name = entry.file_name().to_str().map_err(bad)?;
            if !name.starts_with("candidate-") {
                continue;
            }
            let raw = read(&self.directory, name)?;
            let candidate = Snapshot::parse(&raw, self.limits).map_err(bad)?;
            let contains = candidate.body()["records"]
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .any(|r| r["kind"] == "ActivationIntent" && r["id"] == id);
            if !contains {
                continue;
            }
            if candidate
                .pending()
                .and_then(|p| candidate.record(p).ok())
                .is_some_and(|p| {
                    p["body"]["stage"] == "Preparing" && p["body"]["intent"]["id"] == id
                })
            {
                activation::validate_transition(
                    &self.current,
                    &candidate,
                    self.current.sha256(),
                    self.current.body()["authority_scope_sha256"]
                        .as_str()
                        .ok_or_else(invalid)?,
                )
                .map_err(bad)?;
                if result
                    .as_ref()
                    .is_some_and(|prior| prior.bytes() != candidate.bytes())
                {
                    return Err(invalid());
                }
                result = Some(candidate);
            }
        }
        Ok(result)
    }
    fn trip(&mut self, point: &str) -> IoResult<()> {
        if self.faults.remove(&format!("crash:{point}")) {
            std::process::exit(86);
        }
        if self.faults.remove(point) {
            return Err(std::io::Error::other(format!(
                "registered disposable fault: {point}"
            )));
        }
        Ok(())
    }
    fn synchronize(&mut self) -> IoResult<()> {
        self.pins()?;
        let raw = read(&self.directory, SNAPSHOT)?;
        if raw == self.current.bytes() {
            barrier(&regular(&self.directory, SNAPSHOT, false)?)?;
            barrier(&self.directory)?;
        } else {
            if self.uncertain_candidate.as_deref() != Some(raw.as_slice()) {
                return Err(std::io::Error::other(
                    "activation selector is neither exact old nor original candidate",
                ));
            }
            let observed = Snapshot::parse(&raw, self.limits).map_err(bad)?;
            activation::validate_transition(
                &self.current,
                &observed,
                self.current.sha256(),
                self.current.body()["authority_scope_sha256"]
                    .as_str()
                    .ok_or_else(invalid)?,
            )
            .map_err(bad)?;
            barrier(&regular(&self.directory, SNAPSHOT, false)?)?;
            barrier(&self.directory)?;
            self.current = observed;
            self.uncertain_candidate = None;
        }
        Ok(())
    }
    fn footprint(&self, candidate: &str, planned: u64) -> IoResult<()> {
        let mut total = number(&self.registration["namespace_allowance_bytes"])?;
        let mut count = 0;
        let mut exists = false;
        for entry in rustix::fs::Dir::read_from(&self.directory)? {
            let entry = entry?;
            let name = entry.file_name().to_str().map_err(bad)?;
            if name == "." || name == ".." {
                continue;
            }
            count += 1;
            if count > self.limits.max_records + 3 {
                return Err(invalid());
            }
            if name != ".writer-lock" && name != SNAPSHOT && !name.starts_with("candidate-") {
                return Err(invalid());
            }
            let f = regular(&self.directory, name, false)?;
            let m = f.metadata()?;
            let charge = m
                .len()
                .max(m.blocks().checked_mul(512).ok_or_else(invalid)?)
                .max(1)
                .div_ceil(4096)
                .checked_mul(4096)
                .ok_or_else(invalid)?;
            total = total.checked_add(charge).ok_or_else(invalid)?;
            if name == candidate {
                exists = true;
            }
        }
        let d = self.directory.metadata()?;
        total = total
            .checked_add(
                d.len()
                    .max(d.blocks().checked_mul(512).ok_or_else(invalid)?)
                    .max(1)
                    .div_ceil(4096)
                    * 4096,
            )
            .ok_or_else(invalid)?;
        if !exists {
            total = total
                .checked_add(planned.max(1).div_ceil(4096) * 4096)
                .ok_or_else(invalid)?;
        }
        if total > number(&self.registration["budget_bytes"])? {
            return Err(std::io::Error::other(
                "registered activation capacity exhausted",
            ));
        }
        Ok(())
    }
    fn open(path: &Path, binding: Value, bindings: Value) -> IoResult<Self> {
        let scope = Scope::load_explicit(path, binding)?;
        let raw = read(&scope.scratch, "activation-registration.json")?;
        let reg: Value = serde_json::from_slice(&raw).map_err(bad)?;
        if canonical(&reg) != raw || reg["version"] != 1 {
            return Err(invalid());
        }
        for field in ["device_id", "workspace_slot_id"] {
            package::PackageUuid::parse(reg[field].as_str().ok_or_else(invalid)?).map_err(bad)?;
        }
        let directory = File::from(openat(
            &scope.mount,
            "activation",
            DIRECTORY,
            Mode::empty(),
        )?);
        let lock = regular(&directory, ".writer-lock", true)?;
        if reg["directory_pin"] != json!(pin(&directory)?)
            || reg["lock_pin"] != json!(pin(&lock)?)
            || directory.metadata()?.permissions().mode() & 0o077 != 0
        {
            return Err(invalid());
        }
        flock(&lock, FlockOperation::NonBlockingLockExclusive)?;
        let max = usize::try_from(number(&reg["max_snapshot_bytes"])?).map_err(bad)?;
        let max_records = usize::try_from(number(&reg["max_records"])?).map_err(bad)?;
        let limits = activation::Limits {
            json: package::JsonLimits {
                max_bytes: max,
                max_depth: 128,
                max_members: 4096,
                max_array_elements: 100_000,
            },
            max_records,
            max_snapshot_bytes: max,
        };
        if max == 0
            || max > 16 * 1024 * 1024
            || max_records == 0
            || max_records > 4096
            || number(&reg["budget_bytes"])?
                < (max as u64)
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(number(&reg["namespace_allowance_bytes"]).ok()?))
                    .ok_or_else(invalid)?
        {
            return Err(invalid());
        }
        let targets = reg["targets"]
            .as_array()
            .filter(|v| !v.is_empty() && v.len() <= 16)
            .ok_or_else(invalid)?;
        let mut ids = std::collections::BTreeSet::new();
        for target in targets {
            let id = target["project_id"].as_str().ok_or_else(invalid)?;
            if !ids.insert(id)
                || target["library_id"] != targets[0]["library_id"]
                || bindings[id].is_null()
            {
                return Err(invalid());
            }
            for key in ["library_id", "project_id", "incarnation_id", "graph_id"] {
                package::PackageUuid::parse(target[key].as_str().ok_or_else(invalid)?)
                    .map_err(bad)?;
            }
            package::Sha256Hex::parse(target["registration_sha256"].as_str().ok_or_else(invalid)?)
                .map_err(bad)?;
        }
        let current = Snapshot::parse(&read(&directory, SNAPSHOT)?, limits).map_err(bad)?;
        if current.body()["authority_scope_sha256"] != hash(&raw)
            || current.body()["device_id"] != reg["device_id"]
            || current.body()["workspace_slot_id"] != reg["workspace_slot_id"]
        {
            return Err(invalid());
        }
        Ok(Self {
            scope,
            directory,
            lock,
            registration: reg,
            registration_bytes: raw,
            bindings,
            current,
            limits,
            faults: std::collections::BTreeSet::default(),
            uncertain_candidate: None,
        })
    }
    fn pins(&self) -> IoResult<()> {
        self.scope.assessment.check_pins().map_err(bad)?;
        if read(&self.scope.scratch, "activation-registration.json")? != self.registration_bytes
            || pin(&self.directory)?
                != pin(&File::from(openat(
                    &self.scope.mount,
                    "activation",
                    DIRECTORY,
                    Mode::empty(),
                )?))?
            || json!(pin(&self.lock)?) != self.registration["lock_pin"]
            || pin(&regular(&self.directory, ".writer-lock", false)?)? != pin(&self.lock)?
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn publish(&mut self, mut body: Value) -> IoResult<()> {
        body["revision"] = json!(
            number(&self.current.body()["revision"])?
                .checked_add(1)
                .ok_or_else(invalid)?
                .to_string()
        );
        let next = Snapshot::from_body(body, self.limits).map_err(bad)?;
        activation::validate_transition(
            &self.current,
            &next,
            self.current.sha256(),
            self.current.body()["authority_scope_sha256"]
                .as_str()
                .ok_or_else(invalid)?,
        )
        .map_err(bad)?;
        self.pins()?;
        let actual = read(&self.directory, SNAPSHOT)?;
        if actual == next.bytes() {
            barrier(&regular(&self.directory, SNAPSHOT, false)?)?;
            barrier(&self.directory)?;
            self.current = next;
            return Ok(());
        }
        if actual != self.current.bytes() {
            return Err(invalid());
        }
        let name = format!("candidate-{}", next.sha256());
        let phase = if next.body()["active"] == self.current.body()["active"] {
            "progress"
        } else {
            "commit"
        };
        let needed = (self.current.bytes().len() as u64)
            .checked_add(next.bytes().len() as u64)
            .and_then(|n| {
                n.checked_add(number(&self.registration["namespace_allowance_bytes"]).ok()?)
            })
            .ok_or_else(invalid)?;
        if needed > number(&self.registration["budget_bytes"])? {
            return Err(invalid());
        }
        self.footprint(&name, self.limits.max_snapshot_bytes as u64)?;
        self.uncertain_candidate = Some(next.bytes().to_vec());
        self.trip(&format!("{phase}:before-create"))?;
        let candidate = match regular(&self.directory, &name, true) {
            Ok(f) => {
                if read(&self.directory, &name)? != next.bytes() {
                    return Err(invalid());
                }
                f
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut f = File::from(openat(
                    &self.directory,
                    &name,
                    OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::RDWR
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )?);
                f.write_all(next.bytes())?;
                f
            }
            Err(e) => return Err(e),
        };
        self.trip(&format!("{phase}:after-write"))?;
        barrier(&candidate)?;
        self.trip(&format!("{phase}:after-file-barrier"))?;
        barrier(&self.directory)?;
        self.trip(&format!("{phase}:after-candidate-directory-barrier"))?;
        self.footprint(&name, 0)?;
        self.pins()?;
        if read(&self.directory, SNAPSHOT)? != self.current.bytes() {
            return Err(invalid());
        }
        self.trip(&format!("{phase}:before-rename"))?;
        renameat(&self.directory, &name, &self.directory, SNAPSHOT)?;
        self.trip(&format!("{phase}:after-rename"))?;
        barrier(&self.directory)?;
        self.trip(&format!("{phase}:after-directory-barrier"))?;
        if read(&self.directory, SNAPSHOT)? != next.bytes() {
            return Err(invalid());
        }
        self.current = next;
        self.uncertain_candidate = None;
        Ok(())
    }
    fn target(&self, id: &str) -> IoResult<Value> {
        let t = self.registration["targets"]
            .as_array()
            .ok_or_else(invalid)?
            .iter()
            .find(|v| v["project_id"] == id)
            .ok_or_else(invalid)?;
        Ok(
            json!({"library_id":t["library_id"],"project_id":t["project_id"],"incarnation_id":t["incarnation_id"],"graph_id":t["graph_id"],"registration_sha256":t["registration_sha256"]}),
        )
    }
    fn open_target(&self, id: &str) -> IoResult<session_host::OpenSession> {
        self.pins()?;
        let t = self.registration["targets"]
            .as_array()
            .ok_or_else(invalid)?
            .iter()
            .find(|v| v["project_id"] == id)
            .ok_or_else(invalid)?;
        let path = Path::new(t["manifest_path"].as_str().ok_or_else(invalid)?);
        let mut opened = super::open_controlled(path, self.bindings[id].clone())?;
        if opened.activation_target()? != self.target(id)?
            || opened.activation_device() != self.registration["device_id"]
        {
            return Err(invalid());
        }
        Ok(opened)
    }
}

pub(in crate::package::v1_3::repeatable::disposable) struct Coordinator {
    storage: Storage,
    session: Option<session_host::OpenSession>,
    recovery: Option<Value>,
}
impl Coordinator {
    pub(in crate::package::v1_3::repeatable::disposable) fn open(
        path: &Path,
        binding: Value,
        bindings: Value,
    ) -> IoResult<Self> {
        let storage = Storage::open(path, binding, bindings)?;
        let mut this = Self {
            storage,
            session: None,
            recovery: None,
        };
        this.restore_current();
        Ok(this)
    }
    fn restore_current(&mut self) {
        self.session = None;
        let Some(active) = self.storage.current.active().cloned() else {
            return;
        };
        let Ok(record) = self.storage.current.record(&active) else {
            return;
        };
        let Some(id) = record["body"]["project_id"].as_str() else {
            return;
        };
        let id = id.to_owned();
        let pending_saved = self
            .storage
            .current
            .pending()
            .and_then(|p| self.storage.current.record(p).ok())
            .map(|r| r["body"]["source_saved"].clone())
            .filter(|r| !r.is_null());
        let recovery_saved = self.storage.current.body()["records"]
            .as_array()
            .and_then(|rows| {
                rows.iter().find(|r| {
                    r["kind"] == "ActivationReceipt"
                        && r["body"]["outcome"] == "RetainedReadOnlyRecovery"
                        && r["body"]["active"] == active
                        && self
                            .storage
                            .current
                            .record(&r["body"]["intent"])
                            .is_ok_and(|i| {
                                i["body"]["request_generation"]
                                    == self.storage.current.body()["request_generation"]
                            })
                })
            })
            .map(|r| r["body"]["source_saved"].clone())
            .filter(|r| !r.is_null());
        let expected = pending_saved
            .or(recovery_saved)
            .and_then(|r| self.storage.current.record(&r).ok())
            .map(|r| r["body"].clone());
        match self
            .storage
            .trip("source-reacquire")
            .and_then(|()| self.storage.open_target(&id))
            .and_then(|mut session| {
                if let Some(expected) = expected {
                    let actual = session.activation_observe(&self.storage.target(&id)?)?;
                    if actual["saved"]["accepted"] != expected["accepted"]
                        || actual["head"]
                            != expected["checkpoint_receipt"]["value"]["body"]["selected_head"]
                    {
                        return Err(std::io::Error::other(
                            "source changed since verified rollback checkpoint",
                        ));
                    }
                }
                Ok(session)
            }) {
            Ok(s) => {
                self.session = Some(s);
                self.recovery = None;
            }
            Err(e) => {
                let capsule = self.storage.current.body()["records"]
                    .as_array()
                    .and_then(|rows| {
                        rows.iter()
                            .filter(|r| r["kind"] == "ActivationIntent")
                            .filter_map(|intent| {
                                let generation =
                                    number(&intent["body"]["request_generation"]).ok()?;
                                let capsule_id = rid(intent["id"].as_str()?, "capsule");
                                let capsule = rows.iter().find(|r| r["id"] == capsule_id)?;
                                // The latest capsule also remains available when the committed
                                // target cannot reopen; it is explicitly a source recovery view.
                                Some((generation, capsule))
                            })
                            .max_by_key(|(generation, _)| *generation)
                            .map(|(_, r)| r)
                    })
                    .cloned();
                let activation_id = capsule
                    .as_ref()
                    .and_then(|c| activation::record_ref(c).ok())
                    .and_then(|r| {
                        self.storage.current.body()["records"]
                            .as_array()?
                            .iter()
                            .find(|v| v["kind"] == "ActivationReceipt" && v["body"]["capsule"] == r)
                    })
                    .map(|r| r["body"]["intent"]["id"].clone());
                self.recovery = Some(
                    json!({"error":e.to_string(),"capsule":capsule,"activation_id":activation_id}),
                );
            }
        }
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn inject_faults(
        &mut self,
        points: &[String],
    ) -> IoResult<()> {
        if self.storage.registration["allow_faults"] != true || points.len() > 4 {
            return Err(invalid());
        }
        for point in points {
            let valid = matches!(
                point.as_str(),
                "crash:commit:before-rename" | "crash:commit:after-rename"
            ) || matches!(
                point.as_str(),
                "source-save" | "target-open" | "target-restore" | "source-reacquire"
            ) || ["progress", "commit"].iter().any(|phase| {
                [
                    "before-create",
                    "after-write",
                    "after-file-barrier",
                    "after-candidate-directory-barrier",
                    "before-rename",
                    "after-rename",
                    "after-directory-barrier",
                ]
                .iter()
                .any(|cut| point == &format!("{phase}:{cut}"))
            });
            if !valid {
                return Err(invalid());
            }
        }
        self.storage.faults.extend(points.iter().cloned());
        Ok(())
    }
    fn state(&self, status: &str, id: Option<&str>, error: Option<String>) -> Value {
        let active = self
            .storage
            .current
            .active()
            .and_then(|r| self.storage.current.record(r).ok());
        let pending = self
            .storage
            .current
            .pending()
            .and_then(|r| self.storage.current.record(r).ok());
        let (current, display_error) = match self
            .session
            .as_ref()
            .map(session_host::OpenSession::activation_display)
            .transpose()
        {
            Ok(value) => (value, None),
            Err(e) => (None, Some(e.to_string())),
        };
        let error = error.or(display_error);
        let title = |project: &Value| {
            self.storage.registration["targets"]
                .as_array()
                .and_then(|rows| rows.iter().find(|r| r["project_id"] == *project))
                .map(|r| r.get("title").unwrap_or(&r["project_id"]).clone())
        };
        let intent = pending.and_then(|p| self.storage.current.record(&p["body"]["intent"]).ok());
        json!({"status":if self.recovery.is_some(){"RetainedReadOnlyRecovery"}else if pending.is_some()&&status=="Current"{"Pending"}else{status},"activation_id":id.map(str::to_owned).or_else(||intent.and_then(|r|r["id"].as_str()).map(str::to_owned)).or_else(||self.recovery.as_ref().and_then(|r|r["activation_id"].as_str()).map(str::to_owned)),"pending_target_project_id":intent.map(|r|&r["body"]["target"]["project_id"]),"active":active,"pending":pending,"current":current,"recovery":self.recovery,"error":error,"snapshot_sha256":self.storage.current.sha256(),"view":active.and_then(|r|self.storage.current.record(&r["body"]["view"]).ok()).map(|r|&r["body"]),"projects":self.storage.registration["targets"],"confirmation_required":intent.is_some_and(|i|!i["body"]["source_active"].is_null()),"source_title":active.and_then(|r|title(&r["body"]["project_id"])),"target_title":intent.and_then(|r|title(&r["body"]["target"]["project_id"])),"read_only":self.recovery.is_some(),"recovery_nodes":self.recovery.as_ref().and_then(|r|r["capsule"]["body"]["graph_snapshot"]["nodes"].as_array()).map(|nodes|nodes.iter().filter_map(|n|Some(json!({"id":n["id"],"title":n["definition"]["definition_id"].as_str()?,"x":n["photara.graph-position"]["x"].as_i64()?,"y":n["photara.graph-position"]["y"].as_i64()?}))).collect::<Vec<_>>())})
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn execute(
        &mut self,
        request: &Value,
    ) -> Value {
        if self.storage.pins().is_err()
            || !read(&self.storage.directory, SNAPSHOT)
                .is_ok_and(|raw| raw == self.storage.current.bytes())
        {
            return json!({"error":"activation authority changed; no mutation performed"});
        }
        if self.storage.current.pending().is_some() {
            return self.state(
                "Pending",
                None,
                Some("activation pending; editor frozen".into()),
            );
        }
        self.session.as_mut().map_or_else(
            || json!({"error":"No editable current project"}),
            |s| s.execute(request),
        )
    }
    fn pending(&self, id: &str) -> IoResult<(Value, Value)> {
        let p = self
            .storage
            .current
            .record(self.storage.current.pending().ok_or_else(invalid)?)
            .map_err(bad)?
            .clone();
        let intent = self
            .storage
            .current
            .record(&p["body"]["intent"])
            .map_err(bad)?
            .clone();
        if intent["id"] != id {
            return Err(invalid());
        }
        Ok((intent, p))
    }
    fn restored_view(&self, target: &Value, graph: &Value) -> Value {
        let rows = self.storage.current.body()["records"]
            .as_array()
            .expect("validated records");
        let selected = rows
            .iter()
            .filter(|r| r["kind"] == "ActivationIntent")
            .filter_map(|r| {
                let source = self
                    .storage
                    .current
                    .record(&r["body"]["source_active"])
                    .ok()?;
                if source["body"]["project_id"] != target["project_id"] {
                    return None;
                }
                let view_id = rid(r["id"].as_str()?, "source-view");
                let view = rows.iter().find(|v| v["id"] == view_id)?;
                Some((number(&r["body"]["request_generation"]).ok()?, view))
            })
            .max_by_key(|(generation, _)| *generation)
            .map(|(_, v)| v);
        if let Some(v) = selected.filter(|v| {
            activation::validate_view(v, &self.storage.current.body()["device_id"], target, graph)
                && Self::view_supported(&v["body"])
        }) {
            return v["body"].clone();
        }
        activation::default_view(&self.storage.current.body()["device_id"], target)
    }
    fn view_supported(body: &Value) -> bool {
        number(&body["zoom_ppm"]).is_ok_and(|n| (550_000..=1_800_000).contains(&n))
            && body["visible_panels"] == json!(["graph"])
            && body["selected_node_ids"]
                .as_array()
                .is_some_and(|v| v.len() <= 1)
    }
    fn recheck_prompt(
        &mut self,
        intent: &Value,
        basis: &Value,
        epoch: Option<&str>,
        generation: Option<u64>,
    ) -> IoResult<()> {
        if intent["body"]["source_active"].is_null() {
            return Ok(());
        }
        let source = self
            .storage
            .current
            .record(&intent["body"]["source_active"])
            .map_err(bad)?;
        let target = self
            .storage
            .target(source["body"]["project_id"].as_str().ok_or_else(invalid)?)?;
        let session = self.session.as_mut().ok_or_else(invalid)?;
        let binding = session.activation_binding();
        if binding != intent["body"]["source_attachment"]
            || binding != basis["body"]["source_binding"]
            || binding["owner_epoch"].as_str() != epoch
            || generation.map(|n| n.to_string()).as_deref()
                != binding["owner"]["attachment_generation"].as_str()
        {
            return Err(invalid());
        }
        let evidence = session.activation_observe(&target)?;
        if evidence["saved"]["accepted"] != basis["body"]["accepted"]
            || evidence["saved"]["binding"] != binding
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn prepare(
        &mut self,
        id: &str,
        target_id: &str,
        view: Value,
        epoch: Option<&str>,
        generation: Option<u64>,
    ) -> Value {
        let result = self.prepare_inner(id, target_id, view, epoch, generation);
        let outcome = self
            .storage
            .current
            .receipt(id)
            .and_then(|r| r["body"]["outcome"].as_str());
        self.state(
            if result.is_err() {
                "Refused"
            } else {
                outcome.unwrap_or(if self.storage.current.pending().is_some() {
                    "Prepared"
                } else {
                    "Current"
                })
            },
            Some(id),
            result.err().map(|e| e.to_string()),
        )
    }
    #[expect(
        clippy::too_many_lines,
        reason = "Keep ordered native activation effects and rollback proof visible together"
    )]
    fn prepare_inner(
        &mut self,
        id: &str,
        target_id: &str,
        view: Value,
        epoch: Option<&str>,
        generation: Option<u64>,
    ) -> IoResult<()> {
        self.storage.synchronize()?;
        package::PackageUuid::parse(id).map_err(bad)?;
        let target = self.storage.target(target_id)?;
        if let Some(receipt) = self.storage.current.receipt(id) {
            let original = self
                .storage
                .current
                .record(&receipt["body"]["intent"])
                .map_err(bad)?;
            return if original["body"]["target"] == target {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if self.storage.current.pending().is_some() {
            let (intent, p) = self.pending(id)?;
            if intent["body"]["target"] != target || p["body"]["stage"] != "Preparing" {
                return Err(invalid());
            }
            let basis = if intent["body"]["confirmation_basis"].is_null() {
                Value::Null
            } else {
                self.storage
                    .current
                    .record(&intent["body"]["confirmation_basis"])
                    .map_err(bad)?
                    .clone()
            };
            return self.recheck_prompt(&intent, &basis, epoch, generation);
        }
        if let Some(candidate) = self.storage.original_candidate(id)? {
            let progress = candidate
                .record(candidate.pending().ok_or_else(invalid)?)
                .map_err(bad)?;
            let original = candidate.record(&progress["body"]["intent"]).map_err(bad)?;
            if original["body"]["target"] != target {
                return Err(invalid());
            }
            let basis = if original["body"]["confirmation_basis"].is_null() {
                Value::Null
            } else {
                candidate
                    .record(&original["body"]["confirmation_basis"])
                    .map_err(bad)?
                    .clone()
            };
            self.recheck_prompt(original, &basis, epoch, generation)?;
            self.storage.publish(candidate.body().clone())?;
            return Ok(());
        }
        let mut body = self.storage.current.body().clone();
        // Retain all evidence; refuse before selecting a new attempt when the
        // registered record budget cannot hold its complete finite role set.
        if body["records"]
            .as_array()
            .ok_or_else(invalid)?
            .len()
            .checked_add(16)
            .ok_or_else(invalid)?
            > self.storage.limits.max_records
        {
            return Err(std::io::Error::other(
                "activation record reservation exhausted",
            ));
        }
        self.storage.footprint(
            "reserved-next-snapshot",
            self.storage.limits.max_snapshot_bytes as u64,
        )?;
        let active = body["active"].clone();
        if !active.is_null()
            && self.storage.current.record(&active).map_err(bad)?["body"]["project_id"] == target_id
        {
            return Ok(());
        }
        let request = number(&body["request_generation"])?
            .checked_add(1)
            .ok_or_else(invalid)?
            .to_string();
        let mut basis = Value::Null;
        let mut attachment = Value::Null;
        if !active.is_null() {
            let old = self.storage.current.record(&active).map_err(bad)?;
            let source = self
                .storage
                .target(old["body"]["project_id"].as_str().ok_or_else(invalid)?)?;
            self.storage.trip("source-save")?;
            let s = self.session.as_mut().ok_or_else(invalid)?;
            attachment = s.activation_binding();
            if attachment["owner_epoch"].as_str() != epoch
                || generation.map(|n| n.to_string()).as_deref()
                    != attachment["owner"]["attachment_generation"].as_str()
            {
                return Err(invalid());
            }
            let evidence = s.activation_flush(&source)?;
            let future = (canonical(&body).len() as u64)
                .checked_add(
                    (canonical(&evidence["saved"]).len() as u64)
                        .checked_mul(2)
                        .ok_or_else(invalid)?,
                )
                .and_then(|n| n.checked_add(canonical(&evidence["graph"]).len() as u64))
                .and_then(|n| n.checked_add(131_072))
                .ok_or_else(invalid)?;
            // The disposable surface has one fixed Graph and fixed-width
            // wrapper fields; reserve 128 KiB for the remaining view, target,
            // progress and receipt records. Actual snapshots remain bounded and
            // independently checked before every candidate write.
            if future > self.storage.limits.max_snapshot_bytes as u64 {
                return Err(std::io::Error::other(
                    "activation snapshot reservation exhausted",
                ));
            }
            let saved = put!(
                &mut body,
                record(
                    &rid(id, "prior-saved"),
                    "SavedProof",
                    evidence["saved"].clone()
                )
            )?;
            let mut v = record(&rid(id, "source-view"), "SessionView", view);
            if !activation::validate_view(&v, &body["device_id"], &source, &evidence["graph"])
                || !Self::view_supported(&v["body"])
            {
                v["body"] = activation::default_view(&body["device_id"], &source);
            }
            put!(&mut body, v)?;
            basis = put!(
                &mut body,
                record(
                    &rid(id, "confirmation"),
                    "ConfirmationEvidence",
                    json!({"activation_id":id,"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"request_generation":request,"expected_committed_generation":body["committed_generation"],"authority_scope_sha256":body["authority_scope_sha256"],"source_active":active,"source_binding":attachment,"accepted":evidence["saved"]["accepted"],"prior_saved":saved,"target":target})
                )
            )?;
        }
        let intent = record(
            id,
            "ActivationIntent",
            json!({"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"request_generation":request,"expected_committed_generation":body["committed_generation"],"authority_scope_sha256":body["authority_scope_sha256"],"source_active":active,"target":target,"source_attachment":attachment,"confirmation_basis":basis}),
        );
        let intent_ref = put!(&mut body, intent)?;
        body["request_generation"] = json!(request);
        body["pending"] = put!(
            &mut body,
            record(
                &rid(id, "Preparing"),
                "ActivationProgress",
                json!({"intent":intent_ref,"stage":"Preparing","source_saved":null,"capsule":null,"target_open":null})
            )
        )?;
        self.storage.publish(body)
    }
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Owned immutable evidence crosses a serialized snapshot construction boundary"
    )]
    fn progress(
        &mut self,
        id: &str,
        stage: &str,
        saved: Value,
        capsule: Value,
        target: Value,
        extra: Vec<Value>,
    ) -> IoResult<()> {
        let (intent, _) = self.pending(id)?;
        let mut body = self.storage.current.body().clone();
        for r in extra {
            put!(&mut body, r)?;
        }
        body["pending"] = put!(
            &mut body,
            record(
                &rid(id, stage),
                "ActivationProgress",
                json!({"intent":activation::record_ref(&intent).map_err(bad)?,"stage":stage,"source_saved":saved,"capsule":capsule,"target_open":target})
            )
        )?;
        self.storage.publish(body)
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn confirm(&mut self, id: &str) -> Value {
        let result = self.confirm_inner(id);
        if let Err(error) = result {
            let synchronized = self.storage.synchronize().is_ok();
            if synchronized {
                if let Some(receipt) = self.storage.current.receipt(id) {
                    if receipt["body"]["outcome"] == "Activated" {
                        self.restore_current();
                    }
                } else if let Ok((_, p)) = self.pending(id) {
                    if self.session.is_none() {
                        self.restore_current();
                    }
                    let _ = self.refuse(
                        id,
                        p["body"]["source_saved"].clone(),
                        p["body"]["capsule"].clone(),
                    );
                }
            }
            let outcome = self
                .storage
                .current
                .receipt(id)
                .and_then(|r| r["body"]["outcome"].as_str())
                .unwrap_or("Refused");
            let reconciled = synchronized
                && outcome == "Activated"
                && self.session.is_some()
                && self.recovery.is_none();
            return self.state(
                outcome,
                Some(id),
                if reconciled {
                    None
                } else {
                    Some(error.to_string())
                },
            );
        }
        self.state(
            self.storage
                .current
                .receipt(id)
                .and_then(|r| r["body"]["outcome"].as_str())
                .unwrap_or("Refused"),
            Some(id),
            None,
        )
    }
    #[expect(
        clippy::too_many_lines,
        reason = "Keep ordered native activation effects and rollback proof visible together"
    )]
    fn confirm_inner(&mut self, id: &str) -> IoResult<()> {
        self.storage.synchronize()?;
        if self.storage.current.receipt(id).is_some() {
            return Ok(());
        }
        let (intent, p) = self.pending(id)?;
        let ib = &intent["body"];
        if p["body"]["stage"] != "Preparing" {
            return Err(invalid());
        }
        if !ib["source_active"].is_null()
            && self
                .session
                .as_ref()
                .ok_or_else(invalid)?
                .activation_binding()
                != ib["source_attachment"]
        {
            return Err(invalid());
        }
        if !ib["source_active"].is_null() {
            self.progress(
                id,
                "Confirmed",
                Value::Null,
                Value::Null,
                Value::Null,
                vec![],
            )?;
            self.progress(id, "Frozen", Value::Null, Value::Null, Value::Null, vec![])?;
        }
        let mut saved = Value::Null;
        let mut capsule = Value::Null;
        if !ib["source_active"].is_null() {
            self.storage.trip("source-save")?;
            let active = self
                .storage
                .current
                .record(&ib["source_active"])
                .map_err(bad)?;
            let source = self
                .storage
                .target(active["body"]["project_id"].as_str().ok_or_else(invalid)?)?;
            let evidence = self
                .session
                .as_mut()
                .ok_or_else(invalid)?
                .activation_flush(&source)?;
            let basis = self
                .storage
                .current
                .record(&ib["confirmation_basis"])
                .map_err(bad)?;
            if basis["body"]["source_binding"] != evidence["saved"]["binding"]
                || basis["body"]["accepted"] != evidence["saved"]["accepted"]
            {
                return Err(invalid());
            }
            let sr = record(
                &rid(id, "source-saved"),
                "SavedProof",
                evidence["saved"].clone(),
            );
            saved = activation::record_ref(&sr).map_err(bad)?;
            let view = self.storage.current.body()["records"]
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .find(|r| r["id"] == rid(id, "source-view"))
                .ok_or_else(invalid)?;
            let cr = record(
                &rid(id, "capsule"),
                "RollbackCapsule",
                json!({"source_active":ib["source_active"],"source_saved":saved,"view":activation::record_ref(view).map_err(bad)?,"graph_snapshot":evidence["graph"],"registration_sha256":source["registration_sha256"]}),
            );
            capsule = activation::record_ref(&cr).map_err(bad)?;
            self.progress(
                id,
                "SourceSaved",
                saved.clone(),
                capsule.clone(),
                Value::Null,
                vec![sr, cr],
            )?;
        }
        self.session.take();
        let attempt = (|| -> IoResult<(session_host::OpenSession, Value, Value)> {
            self.storage.trip("target-open")?;
            let target = &ib["target"];
            let mut s = self
                .storage
                .open_target(target["project_id"].as_str().ok_or_else(invalid)?)?;
            let proof = s.activation_flush(target)?;
            let vr = record(
                &rid(id, "target-view"),
                "SessionView",
                self.restored_view(target, &proof["graph"]),
            );
            self.storage.trip("target-restore")?;
            let view = activation::record_ref(&vr).map_err(bad)?;
            let opened = record(
                &rid(id, "target-open"),
                "TargetOpenProof",
                json!({"binding":proof["saved"]["binding"],"registration_sha256":target["registration_sha256"],"selected_head":proof["head"],"selected_commit":proof["commit"],"accepted":proof["saved"]["accepted"],"graph_id":target["graph_id"],"view":view}),
            );
            let openref = activation::record_ref(&opened).map_err(bad)?;
            self.progress(
                id,
                "TargetReady",
                saved.clone(),
                capsule.clone(),
                openref.clone(),
                vec![vr, opened],
            )?;
            // Revalidate live target lease and complete journal proof immediately before publication.
            let again = s.activation_flush(target)?;
            if again != proof {
                return Err(invalid());
            }
            Ok((s, view, openref))
        })();
        let (target_session, view, openref) = match attempt {
            Ok(v) => v,
            Err(e) => {
                self.restore_current();
                self.refuse(id, saved, capsule)?;
                return Err(e);
            }
        };
        let mut body = self.storage.current.body().clone();
        let generation = number(&body["committed_generation"])?
            .checked_add(1)
            .ok_or_else(invalid)?
            .to_string();
        let ar = record(
            &rid(id, "active"),
            "ActiveSession",
            json!({"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"committed_generation":generation,"authority_scope_sha256":body["authority_scope_sha256"],"library_id":ib["target"]["library_id"],"project_id":ib["target"]["project_id"],"incarnation_id":ib["target"]["incarnation_id"],"graph_id":ib["target"]["graph_id"],"activation_id":id,"view":view,"target_open":openref}),
        );
        let active = put!(&mut body, ar)?;
        put!(
            &mut body,
            record(
                &rid(id, "receipt-Activated"),
                "ActivationReceipt",
                json!({"intent":activation::record_ref(&intent).map_err(bad)?,"outcome":"Activated","old_committed_generation":body["committed_generation"],"new_committed_generation":generation,"source_saved":saved,"capsule":capsule,"target_open":openref,"active":active})
            )
        )?;
        body["active"] = active;
        body["pending"] = Value::Null;
        body["committed_generation"] = json!(generation);
        if let Err(e) = self.storage.publish(body) {
            drop(target_session);
            self.restore_current();
            return Err(e);
        }
        self.session = Some(target_session);
        self.recovery = None;
        Ok(())
    }
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Owned immutable evidence crosses a serialized snapshot construction boundary"
    )]
    fn refuse(&mut self, id: &str, saved: Value, capsule: Value) -> IoResult<()> {
        let (intent, p) = self.pending(id)?;
        if p["body"]["stage"] != "Refused" {
            self.progress(
                id,
                "Refused",
                saved.clone(),
                capsule.clone(),
                p["body"]["target_open"].clone(),
                vec![],
            )?;
        }
        let mut body = self.storage.current.body().clone();
        let outcome = if self.session.is_some() || body["active"].is_null() {
            "RetainedCurrent"
        } else {
            "RetainedReadOnlyRecovery"
        };
        put!(
            &mut body,
            record(
                &rid(id, &format!("receipt-{outcome}")),
                "ActivationReceipt",
                json!({"intent":activation::record_ref(&intent).map_err(bad)?,"outcome":outcome,"old_committed_generation":body["committed_generation"],"new_committed_generation":body["committed_generation"],"source_saved":saved,"capsule":capsule,"target_open":p["body"]["target_open"],"active":body["active"]})
            )
        )?;
        body["pending"] = Value::Null;
        self.storage.publish(body)
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn cancel(&mut self, id: &str) -> Value {
        let result = (|| {
            self.storage.synchronize()?;
            if self.storage.current.receipt(id).is_some() {
                return Ok(());
            }
            let (_, p) = self.pending(id)?;
            self.refuse(
                id,
                p["body"]["source_saved"].clone(),
                p["body"]["capsule"].clone(),
            )
        })();
        let outcome = self
            .storage
            .current
            .receipt(id)
            .and_then(|r| r["body"]["outcome"].as_str())
            .unwrap_or("Refused");
        self.state(
            outcome,
            Some(id),
            result.err().map(|e: std::io::Error| e.to_string()),
        )
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn retry(&mut self, id: &str) -> Value {
        // Restart never treats stored owner epochs as current write grants.
        // Until confirmation can be freshly established, retain the original source.
        if let Err(e) = self.storage.synchronize() {
            return self.state("Refused", Some(id), Some(e.to_string()));
        }
        if let Some(receipt) = self.storage.current.receipt(id).cloned() {
            let active_project = self
                .storage
                .current
                .active()
                .and_then(|r| self.storage.current.record(r).ok())
                .map(|r| r["body"]["project_id"].clone());
            let matches = self
                .session
                .as_mut()
                .and_then(|s| s.activation_target().ok())
                .is_some_and(|t| Some(t["project_id"].clone()) == active_project);
            if !matches && self.storage.current.pending().is_none() {
                self.restore_current();
            }
            return self.state(
                receipt["body"]["outcome"].as_str().unwrap_or("Refused"),
                Some(id),
                None,
            );
        }
        if self.storage.current.pending().is_none() {
            match self.storage.original_candidate(id) {
                Ok(Some(candidate)) => {
                    if let Err(e) = self.storage.publish(candidate.body().clone()) {
                        return self.state("Refused", Some(id), Some(e.to_string()));
                    }
                }
                Ok(None) => {}
                Err(e) => return self.state("Refused", Some(id), Some(e.to_string())),
            }
        }
        if let Err(e) = self.pending(id) {
            return self.state("Refused", Some(id), Some(e.to_string()));
        }
        if self.session.is_none() {
            self.restore_current();
        }
        self.cancel(id)
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn snapshot(&self) -> Value {
        self.state("Current", None, None)
    }
}
