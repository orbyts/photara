//! Fresh, registered `SQLite` Library/Project selection; frozen PS4 file slots remain separate.
use super::{
    IoResult, Path, Scope, Value, canonical, invalid, json, number, package, read, session_host,
};
use crate::package::v1_3::activation::{
    self,
    v2::{self, SelectionStore, Snapshot},
};
use std::sync::{Arc, Mutex};
type Database = Arc<Mutex<Box<dyn SelectionStore>>>;
fn bad(evidence: impl std::fmt::Debug) -> std::io::Error {
    std::io::Error::other(format!("local selection refused: {evidence:?}"))
}
fn rid(id: &str, role: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("photara disposable selection {id} {role}").as_bytes());
    let mut body = [0; 16];
    body.copy_from_slice(&digest[..16]);
    body[6] = (body[6] & 15) | 64;
    body[8] = (body[8] & 63) | 128;
    uuid::Uuid::from_bytes(body).to_string()
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "Canonical record builders take ownership of newly constructed bodies."
)]
fn record(id: &str, kind: &str, body: Value) -> Value {
    let view = if matches!(kind, "SavedProof" | "SessionView" | "TargetOpenProof") {
        1
    } else {
        2
    };
    json!({"id":id,"kind":kind,"version":view,"body":body})
}
fn add(body: &mut Value, record: Value) -> IoResult<Value> {
    let reference = v2::record_ref(&record).map_err(bad)?;
    let rows = body["records"].as_array_mut().ok_or_else(invalid)?;
    if let Some(old) = rows.iter().find(|x| x["id"] == record["id"]) {
        if old != &record {
            return Err(invalid());
        }
    } else {
        rows.push(record);
        rows.sort_by(|active, body| active["id"].as_str().cmp(&body["id"].as_str()));
    }
    Ok(reference)
}
fn view_supported(view: &Value) -> bool {
    number(&view["zoom_ppm"]).is_ok_and(|count| (550_000..=1_800_000).contains(&count))
        && view["visible_panels"] == json!(["graph"])
        && view["selected_node_ids"]
            .as_array()
            .is_some_and(|count| count.len() <= 1)
}
pub(in crate::package::v1_3::repeatable::disposable) struct Coordinator {
    scope: Scope,
    registry: Value,
    registry_bytes: Vec<u8>,
    bindings: Value,
    db: Database,
    current: Snapshot,
    limits: v2::Limits,
    session: Option<session_host::OpenSession>,
    recovery: Option<Value>,
    uncertain: Option<Vec<u8>>,
    closed: bool,
}
impl Coordinator {
    pub(in crate::package::v1_3::repeatable::disposable) fn open(
        path: &Path,
        binding: Value,
        bindings: Value,
        db: Box<dyn SelectionStore>,
    ) -> IoResult<Self> {
        let scope = Scope::load_explicit(path, binding)?;
        let raw = read(&scope.scratch, "selection-registration.json")?;
        let registry: Value = serde_json::from_slice(&raw).map_err(bad)?;
        if canonical(&registry) != raw || registry["version"] != 1 {
            return Err(invalid());
        }
        let registration = db.registration();
        for k in [
            "device_id",
            "workspace_slot_id",
            "database_id",
            "epoch",
            "slot_scope_sha256",
        ] {
            if registry[k] != registration[k] {
                return Err(invalid());
            }
        }
        let max = usize::try_from(number(&registration["max_snapshot_bytes"])?).map_err(bad)?;
        let count = usize::try_from(number(&registration["max_records"])?).map_err(bad)?;
        if max == 0 || max > 16 * 1024 * 1024 || count == 0 || count > 4096 {
            return Err(invalid());
        }
        let limits = v2::Limits {
            json: package::JsonLimits {
                max_bytes: max,
                max_depth: 128,
                max_members: 4096,
                max_array_elements: 100_000,
            },
            max_records: count,
            max_snapshot_bytes: max,
        };
        let current = Snapshot::parse(&db.load()?, limits).map_err(bad)?;
        for k in ["device_id", "workspace_slot_id", "slot_scope_sha256"] {
            if current.body()[k] != registration[k] {
                return Err(invalid());
            }
        }
        let mut ids = std::collections::BTreeSet::new();
        for target in registry["targets"]
            .as_array()
            .filter(|target| !target.is_empty() && target.len() <= 16)
            .ok_or_else(invalid)?
        {
            let id = target["project_id"].as_str().ok_or_else(invalid)?;
            if !ids.insert(id) || bindings[id].is_null() {
                return Err(invalid());
            }
            for k in ["project_id", "library_id", "incarnation_id", "graph_id"] {
                package::PackageUuid::parse(target[k].as_str().ok_or_else(invalid)?)
                    .map_err(bad)?;
            }
            package::Sha256Hex::parse(target["registration_sha256"].as_str().ok_or_else(invalid)?)
                .map_err(bad)?;
        }
        let mut session = Self {
            scope,
            registry,
            registry_bytes: raw,
            bindings,
            db: Arc::new(Mutex::new(db)),
            current,
            limits,
            session: None,
            recovery: None,
            uncertain: None,
            closed: false,
        };
        session.restore();
        Ok(session)
    }
    fn pins(&self) -> IoResult<()> {
        self.scope.assessment.check_pins().map_err(bad)?;
        if read(&self.scope.scratch, "selection-registration.json")? != self.registry_bytes {
            return Err(invalid());
        }
        Ok(())
    }
    fn sync(&mut self) -> IoResult<()> {
        self.pins()?;
        let raw = self.db.lock().map_err(bad)?.load()?;
        if raw != self.current.bytes() {
            if self.uncertain.as_deref() != Some(raw.as_slice()) {
                return Err(invalid());
            }
            let next = Snapshot::parse(&raw, self.limits).map_err(bad)?;
            v2::validate_transition(
                &self.current,
                &next,
                self.current.sha256(),
                self.current.body()["slot_scope_sha256"]
                    .as_str()
                    .ok_or_else(invalid)?,
            )
            .map_err(bad)?;
            self.current = next;
        }
        self.uncertain = None;
        Ok(())
    }
    fn publish(&mut self, mut body: Value) -> IoResult<()> {
        self.sync()?;
        body["revision"] = json!(
            number(&self.current.body()["revision"])?
                .checked_add(1)
                .ok_or_else(invalid)?
                .to_string()
        );
        let next = Snapshot::from_body(body, self.limits).map_err(bad)?;
        v2::validate_transition(
            &self.current,
            &next,
            self.current.sha256(),
            self.current.body()["slot_scope_sha256"]
                .as_str()
                .ok_or_else(invalid)?,
        )
        .map_err(bad)?;
        self.uncertain = Some(next.bytes().to_vec());
        self.db
            .lock()
            .map_err(bad)?
            .publish(self.current.sha256(), next.bytes())?;
        self.sync()
    }
    fn context(&self, library: &str) -> IoResult<Value> {
        self.db
            .lock()
            .map_err(bad)?
            .libraries()?
            .into_iter()
            .find(|record| record["id"] == library)
            .map(|record| record["context"].clone())
            .ok_or_else(invalid)
    }
    fn project(&self, id: &str) -> IoResult<Value> {
        let target = self.registry["targets"]
            .as_array()
            .ok_or_else(invalid)?
            .iter()
            .find(|target| target["project_id"] == id)
            .ok_or_else(invalid)?;
        Ok(
            json!({"library_id":target["library_id"],"project_id":target["project_id"],"incarnation_id":target["incarnation_id"],"graph_id":target["graph_id"],"registration_sha256":target["registration_sha256"]}),
        )
    }
    fn project_target(&self, id: &str) -> IoResult<Value> {
        let proof = self.project(id)?;
        Ok(
            json!({"kind":"project","context":self.context(proof["library_id"].as_str().ok_or_else(invalid)?)?,"project":proof}),
        )
    }
    fn open_target(&self, target: &Value) -> IoResult<Option<session_host::OpenSession>> {
        self.pins()?;
        self.db.lock().map_err(bad)?.context(target)?;
        if target["kind"] == "library" {
            return Ok(None);
        }
        let id = target["project"]["project_id"]
            .as_str()
            .ok_or_else(invalid)?;
        if self.project(id)? != target["project"] {
            return Err(invalid());
        }
        let record = self.registry["targets"]
            .as_array()
            .ok_or_else(invalid)?
            .iter()
            .find(|view| view["project_id"] == id)
            .ok_or_else(invalid)?;
        let db = Arc::clone(&self.db);
        let callback_target = target.clone();
        let authority = Arc::new(move || db.lock().map_err(bad)?.provenance(&callback_target));
        let mut session = super::open_local_controlled(
            Path::new(record["manifest_path"].as_str().ok_or_else(invalid)?),
            self.bindings[id].clone(),
            authority,
        )?;
        if session.activation_target()? != target["project"] {
            return Err(invalid());
        }
        if session.activation_device() != self.registry["device_id"] {
            return Err(invalid());
        }
        Ok(Some(session))
    }
    fn active(&self) -> IoResult<Option<Value>> {
        self.current
            .active()
            .map(|active| {
                self.current
                    .record(active)
                    .map(|view| view["body"].clone())
                    .map_err(bad)
            })
            .transpose()
    }
    fn pending(&self, id: &str) -> IoResult<(Value, Value)> {
        let proof = self
            .current
            .record(self.current.pending().ok_or_else(invalid)?)
            .map_err(bad)?
            .clone();
        let intent = self
            .current
            .record(&proof["body"]["intent"])
            .map_err(bad)?
            .clone();
        if intent["id"] != id {
            return Err(invalid());
        }
        Ok((intent, proof))
    }
    fn restore(&mut self) {
        self.session = None;
        let result = (|| {
            let Some(active) = self.active()? else {
                return Ok(None);
            };
            let mut opened = self.open_target(&active["target"])?;
            let saved = self
                .current
                .pending()
                .and_then(|proof| self.current.record(proof).ok())
                .map(|proof| proof["body"]["source_saved"].clone())
                .filter(|record| !record.is_null())
                .or_else(|| {
                    self.current.body()["records"]
                        .as_array()?
                        .iter()
                        .find(|record| {
                            record["kind"] == "ActivationReceipt"
                                && record["body"]["outcome"] == "RetainedReadOnlyRecovery"
                                && record["body"]["active"] == self.current.body()["active"]
                                && self.current.record(&record["body"]["intent"]).is_ok_and(
                                    |intent| {
                                        intent["body"]["request_generation"]
                                            == self.current.body()["request_generation"]
                                    },
                                )
                        })
                        .map(|record| record["body"]["source_saved"].clone())
                        .filter(|record| !record.is_null())
                });
            if let (Some(session), Some(saved)) = (&mut opened, saved) {
                let proof = &self.current.record(&saved).map_err(bad)?["body"];
                let actual = session.activation_observe(&active["target"]["project"])?;
                if actual["saved"]["accepted"] != proof["accepted"]
                    || actual["head"]
                        != proof["checkpoint_receipt"]["value"]["body"]["selected_head"]
                {
                    return Err(invalid());
                }
            }
            Ok(opened)
        })();
        match result {
            Ok(session) => {
                self.session = session;
                self.recovery = None;
            }
            Err(evidence) => {
                let rows = self.current.body()["records"].as_array();
                let capsule = rows.and_then(|rows| {
                    rows.iter()
                        .filter(|record| record["kind"] == "ActivationIntent")
                        .filter_map(|intent| {
                            let id = rid(intent["id"].as_str()?, "capsule");
                            Some((
                                number(&intent["body"]["request_generation"]).ok()?,
                                rows.iter().find(|record| record["id"] == id)?.clone(),
                                intent["id"].clone(),
                            ))
                        })
                        .max_by_key(|record| record.0)
                });
                self.recovery = Some(
                    json!({"error":evidence.to_string(),"capsule":capsule.as_ref().map(|record|&record.1),"activation_id":capsule.map(|record|record.2)}),
                );
            }
        }
    }
    fn state(&self, status: &str, id: Option<&str>, error: Option<String>) -> Value {
        let active = self.active().ok().flatten();
        let selected = active
            .as_ref()
            .map(|active| active["target"]["context"]["library_id"].clone());
        let pending = self
            .current
            .pending()
            .and_then(|proof| self.current.record(proof).ok())
            .and_then(|proof| self.current.record(&proof["body"]["intent"]).ok());
        let display = self
            .session
            .as_ref()
            .map(session_host::OpenSession::activation_display)
            .transpose();
        let display_error = display.as_ref().err().map(ToString::to_string);
        let libraries = self.db.lock().map_err(bad).and_then(|db| db.libraries());
        let library_error = libraries.as_ref().err().map(ToString::to_string);
        let libraries = libraries.unwrap_or_default();
        let projects = self.registry["targets"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter(|target| {
                        selected.as_ref() == Some(&target["library_id"])
                            && self
                                .project_target(target["project_id"].as_str().unwrap_or(""))
                                .is_ok_and(|target| {
                                    self.db.lock().is_ok_and(|db| db.context(&target).is_ok())
                                })
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let creatable = self.registry["targets"].as_array().map(|rows| {
            rows.iter()
                .filter(|target| !libraries.iter().any(|l| l["id"] == target["library_id"]))
                .map(|target| target["library_id"].clone())
                .collect::<Vec<_>>()
        });
        let view = active
            .as_ref()
            .filter(|active| active["target"]["kind"] == "project")
            .and_then(|active| self.current.record(&active["ready"]).ok())
            .and_then(|record| self.current.record(&record["body"]["project_open"]).ok())
            .and_then(|proof| self.current.record(&proof["body"]["view"]).ok())
            .map(|view| view["body"].clone());
        let recovery_nodes = self.recovery.as_ref().and_then(|record| record["capsule"]["body"]["graph_snapshot"]["nodes"].as_array()).map(|nodes| nodes.iter().filter_map(|node| Some(json!({"id":node["id"],"title":node["definition"]["definition_id"].as_str()?,"x":node["photara.graph-position"]["x"].as_i64()?,"y":node["photara.graph-position"]["y"].as_i64()?}))).collect::<Vec<_>>());
        let title = |target: &Value| -> Value {
            if target["kind"] == "project" {
                self.registry["targets"]
                    .as_array()
                    .and_then(|rows| {
                        rows.iter()
                            .find(|proof| proof["project_id"] == target["project"]["project_id"])
                    })
                    .map_or(Value::Null, |proof| proof["title"].clone())
            } else {
                libraries
                    .iter()
                    .find(|l| l["id"] == target["context"]["library_id"])
                    .map_or(Value::Null, |l| l["name"].clone())
            }
        };
        let source_title = active.as_ref().map(|active| title(&active["target"]));
        let target_title = pending.map(|intent| title(&intent["body"]["target"]));
        json!({"source_title":source_title,"target_title":target_title,"status":status,"activation_id":id.map(Value::from).or_else(||pending.map(|intent|intent["id"].clone())).or_else(||self.recovery.as_ref().map(|record|record["activation_id"].clone())),"active":self.current.active().and_then(|record|self.current.record(record).ok()),"pending":self.current.pending().and_then(|record|self.current.record(record).ok()),"pending_target_project_id":pending.map(|intent|&intent["body"]["target"]["project"]["project_id"]),"pending_target_library_id":pending.map(|intent|&intent["body"]["target"]["context"]["library_id"]),"selected_library_id":selected,"libraries":libraries,"projects":projects,"creatable_library_ids":creatable,"current":display.ok().flatten(),"error":error.or(display_error).or(library_error),"recovery":self.recovery,"read_only":self.recovery.is_some(),"view":view,"recovery_nodes":recovery_nodes,"snapshot_sha256":self.current.sha256(),"confirmation_required":pending.is_some_and(|intent|!intent["body"]["confirmation_basis"].is_null()),"closed":self.closed})
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn snapshot(&self) -> Value {
        self.state(
            if self.current.pending().is_some() {
                "Pending"
            } else {
                "Current"
            },
            None,
            None,
        )
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn execute(
        &mut self,
        record: &Value,
    ) -> Value {
        let result = (|| {
            self.sync()?;
            if self.closed || self.current.pending().is_some() || self.recovery.is_some() {
                return Err(invalid());
            }
            let active = self.active()?;
            if let Some(active) = &active {
                self.db.lock().map_err(bad)?.context(&active["target"])?;
            }
            if active
                .as_ref()
                .is_none_or(|active| active["target"]["kind"] == "library")
            {
                if record["command"] == "close" {
                    self.closed = true;
                    return Ok(
                        json!({"id":record["id"],"result":{"context_closed":true},"error":null}),
                    );
                }
                if matches!(
                    record["command"].as_str(),
                    Some("snapshot" | "revalidate" | "barrier")
                ) {
                    return Ok(
                        json!({"id":record["id"],"result":{"context_verified":true},"error":null}),
                    );
                }
                return Err(invalid());
            }
            Ok(self.session.as_mut().ok_or_else(invalid)?.execute(record))
        })();
        result.unwrap_or_else(|evidence| json!({"id":record["id"],"error":evidence.to_string()}))
    }
    fn recheck(
        &mut self,
        intent: &Value,
        epoch: Option<&str>,
        generation: Option<u64>,
    ) -> IoResult<()> {
        if intent["body"]["source_attachment"].is_null() {
            return Ok(());
        }
        let active = self
            .current
            .record(&intent["body"]["source_active"])
            .map_err(bad)?;
        let session = self.session.as_mut().ok_or_else(invalid)?;
        let binding = session.activation_binding();
        if binding != intent["body"]["source_attachment"]
            || binding["owner_epoch"].as_str() != epoch
            || generation.map(|count| count.to_string()).as_deref()
                != binding["owner"]["attachment_generation"].as_str()
        {
            return Err(invalid());
        }
        let evidence = session.activation_observe(&active["body"]["target"]["project"])?;
        if !intent["body"]["confirmation_basis"].is_null() {
            let basis = self
                .current
                .record(&intent["body"]["confirmation_basis"])
                .map_err(bad)?;
            if evidence["saved"]["accepted"] != basis["body"]["accepted"] {
                return Err(invalid());
            }
        }
        Ok(())
    }
    #[expect(
        clippy::too_many_lines,
        reason = "Keep the ordered presave, original identity and first publication checks together."
    )]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "The original target is an owned bounded request value."
    )]
    fn prepare_inner(
        &mut self,
        id: &str,
        target: Value,
        view: Value,
        epoch: Option<&str>,
        generation: Option<u64>,
        preserve_same_library_project: bool,
    ) -> IoResult<()> {
        self.sync()?;
        package::PackageUuid::parse(id).map_err(bad)?;
        if self.closed || self.recovery.is_some() {
            return Err(invalid());
        }
        self.db.lock().map_err(bad)?.context(&target)?;
        if let Some(record) = self.current.receipt(id) {
            let intent = self
                .current
                .record(&record["body"]["intent"])
                .map_err(bad)?;
            if intent["body"]["target"] != target {
                return Err(invalid());
            }
            return Ok(());
        }
        if self.current.pending().is_some() {
            let (intent, proof) = self.pending(id)?;
            if proof["body"]["stage"] != "Preparing" || intent["body"]["target"] != target {
                return Err(invalid());
            }
            return self.recheck(&intent, epoch, generation);
        }
        let old = self.active()?;
        if old.as_ref().is_some_and(|active| {
            active["target"] == target
                || (preserve_same_library_project
                    && target["kind"] == "library"
                    && active["target"]["context"] == target["context"])
        }) {
            return Ok(());
        }
        let mut body = self.current.body().clone();
        if body["records"]
            .as_array()
            .ok_or_else(invalid)?
            .len()
            .checked_add(18)
            .ok_or_else(invalid)?
            > self.limits.max_records
        {
            return Err(invalid());
        }
        let request = number(&body["request_generation"])?
            .checked_add(1)
            .ok_or_else(invalid)?
            .to_string();
        let mut attachment = Value::Null;
        let mut basis = Value::Null;
        if let Some(active) = old
            .as_ref()
            .filter(|active| active["target"]["kind"] == "project")
        {
            let session = self.session.as_mut().ok_or_else(invalid)?;
            attachment = session.activation_binding();
            if attachment["owner_epoch"].as_str() != epoch
                || generation.map(|count| count.to_string()).as_deref()
                    != attachment["owner"]["attachment_generation"].as_str()
            {
                return Err(invalid());
            }
            let evidence = session.activation_flush(&active["target"]["project"])?;
            let future = canonical(&body)
                .len()
                .checked_add(
                    canonical(&evidence["saved"])
                        .len()
                        .checked_mul(2)
                        .ok_or_else(invalid)?,
                )
                .and_then(|count| count.checked_add(canonical(&evidence["graph"]).len()))
                .and_then(|count| count.checked_add(131_072))
                .ok_or_else(invalid)?;
            if future > self.limits.max_snapshot_bytes {
                return Err(invalid());
            }
            let prior = add(
                &mut body,
                record(
                    &rid(id, "prior-saved"),
                    "SavedProof",
                    evidence["saved"].clone(),
                ),
            )?;
            let mut view = record(&rid(id, "source-view"), "SessionView", view);
            if !activation::validate_view(
                &view,
                &body["device_id"],
                &active["target"]["project"],
                &evidence["graph"],
            ) || !view_supported(&view["body"])
            {
                view["body"] =
                    activation::default_view(&body["device_id"], &active["target"]["project"]);
            }
            add(&mut body, view)?;
            if target["kind"] == "project" {
                let capsule = record(
                    &rid(id, "confirmation"),
                    "ConfirmationEvidence",
                    json!({"activation_id":id,"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"request_generation":request,"expected_committed_generation":body["committed_generation"],"slot_scope_sha256":body["slot_scope_sha256"],"source_active":body["active"],"source_binding":attachment,"accepted":evidence["saved"]["accepted"],"prior_saved":prior,"target":target}),
                );
                basis = add(&mut body, capsule)?;
            }
        }
        let intent = record(
            id,
            "ActivationIntent",
            json!({"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"request_generation":request,"expected_committed_generation":body["committed_generation"],"slot_scope_sha256":body["slot_scope_sha256"],"source_active":body["active"],"target":target,"source_attachment":attachment,"confirmation_basis":basis}),
        );
        let intent_ref = add(&mut body, intent)?;
        body["request_generation"] = json!(request);
        body["pending"] = add(
            &mut body,
            record(
                &rid(id, "Preparing"),
                "ActivationProgress",
                json!({"intent":intent_ref,"stage":"Preparing","source_saved":null,"capsule":null,"target_ready":null}),
            ),
        )?;
        self.publish(body)
    }
    fn outcome(&self, id: &str) -> &str {
        self.current
            .receipt(id)
            .and_then(|record| record["body"]["outcome"].as_str())
            .unwrap_or(if self.current.pending().is_some() {
                "Prepared"
            } else {
                "Current"
            })
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn prepare(
        &mut self,
        id: &str,
        project: &str,
        view: Value,
        epoch: Option<&str>,
        generation: Option<u64>,
    ) -> Value {
        let result = self
            .project_target(project)
            .and_then(|target| self.prepare_inner(id, target, view, epoch, generation, true));
        match result {
            Ok(()) => self.state(self.outcome(id), Some(id), None),
            Err(evidence) => self.state("Refused", Some(id), Some(evidence.to_string())),
        }
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn select(
        &mut self,
        id: &str,
        library: &str,
        view: Value,
        epoch: Option<&str>,
        generation: Option<u64>,
    ) -> Value {
        let result = self.context(library).and_then(|capsule| {
            self.prepare_inner(
                id,
                json!({"kind":"library","context":capsule}),
                view,
                epoch,
                generation,
                true,
            )
        });
        if let Err(evidence) = result {
            return self.state("Refused", Some(id), Some(evidence.to_string()));
        }
        if self.current.pending().is_some() {
            self.confirm(id)
        } else {
            self.state(self.outcome(id), Some(id), None)
        }
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn close_project(
        &mut self,
        id: &str,
        view: Value,
        epoch: Option<&str>,
        generation: Option<u64>,
    ) -> Value {
        let result = (|| {
            self.sync()?;
            // A retry keeps its original Library target even after publication.
            let target = if let Some(receipt) = self.current.receipt(id) {
                self.current
                    .record(&receipt["body"]["intent"])
                    .map_err(bad)?["body"]["target"]
                    .clone()
            } else if self.current.pending().is_some() {
                self.pending(id)?.0["body"]["target"].clone()
            } else {
                let active = self.active()?.ok_or_else(invalid)?;
                json!({"kind":"library","context":active["target"]["context"]})
            };
            if target["kind"] != "library" {
                return Err(invalid());
            }
            self.prepare_inner(id, target, view, epoch, generation, false)
        })();
        if let Err(error) = result {
            return self.state("Refused", Some(id), Some(error.to_string()));
        }
        if self.current.pending().is_some() {
            self.confirm(id)
        } else {
            self.state(self.outcome(id), Some(id), None)
        }
    }
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Each publication receives owned immutable evidence values."
    )]
    fn progress(
        &mut self,
        id: &str,
        stage: &str,
        saved: Value,
        capsule: Value,
        ready: Value,
        extra: Vec<Value>,
    ) -> IoResult<()> {
        let (intent, _) = self.pending(id)?;
        let mut body = self.current.body().clone();
        for record in extra {
            add(&mut body, record)?;
        }
        body["pending"] = add(
            &mut body,
            record(
                &rid(id, stage),
                "ActivationProgress",
                json!({"intent":v2::record_ref(&intent).map_err(bad)?,"stage":stage,"source_saved":saved,"capsule":capsule,"target_ready":ready}),
            ),
        )?;
        self.publish(body)
    }
    fn restored_view(&self, target: &Value, graph: &Value) -> Value {
        let rows = self.current.body()["records"].as_array();
        if let Some(rows) = rows {
            let mut views = rows
                .iter()
                .filter(|intent| intent["kind"] == "ActivationIntent")
                .filter_map(|intent| {
                    let id = rid(intent["id"].as_str()?, "source-view");
                    Some((
                        number(&intent["body"]["request_generation"]).ok()?,
                        rows.iter().find(|record| record["id"] == id)?,
                    ))
                })
                .collect::<Vec<_>>();
            views.sort_by_key(|view| std::cmp::Reverse(view.0));
            for (_, view) in views {
                if activation::validate_view(view, &self.current.body()["device_id"], target, graph)
                    && view_supported(&view["body"])
                {
                    return view["body"].clone();
                }
            }
        }
        activation::default_view(&self.current.body()["device_id"], target)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "The ordered freeze, evidence, provisional open and publication sequence stays explicit."
    )]
    fn confirm_inner(&mut self, id: &str) -> IoResult<()> {
        self.sync()?;
        if self.current.receipt(id).is_some() {
            return Ok(());
        }
        let (intent_record, proof) = self.pending(id)?;
        if proof["body"]["stage"] != "Preparing" {
            return Err(invalid());
        }
        let intent_ref = v2::record_ref(&intent_record).map_err(bad)?;
        let intent = &intent_record["body"];
        let mut saved = Value::Null;
        let mut capsule = Value::Null;
        if !intent["source_attachment"].is_null() {
            let binding = self
                .session
                .as_ref()
                .ok_or_else(invalid)?
                .activation_binding();
            let epoch = binding["owner_epoch"].as_str();
            let generation = number(&binding["owner"]["attachment_generation"])?;
            self.recheck(&intent_record, epoch, Some(generation))?;
            if !intent["confirmation_basis"].is_null() {
                self.progress(
                    id,
                    "Confirmed",
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    vec![],
                )?;
            }
            self.progress(id, "Frozen", Value::Null, Value::Null, Value::Null, vec![])?;
            let active = self.current.record(&intent["source_active"]).map_err(bad)?;
            let evidence = self
                .session
                .as_mut()
                .ok_or_else(invalid)?
                .activation_flush(&active["body"]["target"]["project"])?;
            let sr = record(
                &rid(id, "source-saved"),
                "SavedProof",
                evidence["saved"].clone(),
            );
            saved = v2::record_ref(&sr).map_err(bad)?;
            let view = self.current.body()["records"]
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .find(|record| record["id"] == rid(id, "source-view"))
                .ok_or_else(invalid)?;
            let cr = record(
                &rid(id, "capsule"),
                "RollbackCapsule",
                json!({"source_active":intent["source_active"],"source_saved":saved,"view":v2::record_ref(view).map_err(bad)?,"graph_snapshot":evidence["graph"],"registration_sha256":evidence["saved"]["registration_sha256"]}),
            );
            capsule = v2::record_ref(&cr).map_err(bad)?;
            self.progress(
                id,
                "SourceSaved",
                saved.clone(),
                capsule.clone(),
                Value::Null,
                vec![sr, cr],
            )?;
        }
        self.session = None;
        let target = &intent["target"];
        let mut opened = self.open_target(target)?;
        let mut extra = Vec::new();
        let mut project_open = Value::Null;
        if let Some(session) = &mut opened {
            let evidence = session.activation_flush(&target["project"])?;
            let view = record(
                &rid(id, "target-view"),
                "SessionView",
                self.restored_view(&target["project"], &evidence["graph"]),
            );
            let vr = v2::record_ref(&view).map_err(bad)?;
            let proof = record(
                &rid(id, "target-open"),
                "TargetOpenProof",
                json!({"binding":evidence["saved"]["binding"],"registration_sha256":evidence["saved"]["registration_sha256"],"selected_head":evidence["head"],"selected_commit":evidence["commit"],"accepted":evidence["saved"]["accepted"],"graph_id":target["project"]["graph_id"],"view":vr}),
            );
            project_open = v2::record_ref(&proof).map_err(bad)?;
            extra.extend([view, proof]);
        }
        let cp = record(
            &rid(id, "context-proof"),
            "LocalContextProof",
            self.db.lock().map_err(bad)?.context(target)?,
        );
        let cpr = v2::record_ref(&cp).map_err(bad)?;
        let ready = record(
            &rid(id, "ready"),
            "SelectionReady",
            json!({"intent":intent_ref,"context_proof":cpr,"project_open":project_open}),
        );
        let rr = v2::record_ref(&ready).map_err(bad)?;
        extra.extend([cp, ready]);
        self.progress(
            id,
            "TargetReady",
            saved.clone(),
            capsule.clone(),
            rr.clone(),
            extra,
        )?;
        if let Some(session) = &mut opened {
            let actual = session.activation_observe(&target["project"])?;
            let proof = self.current.record(&project_open).map_err(bad)?;
            if actual["saved"]["binding"] != proof["body"]["binding"]
                || actual["saved"]["accepted"] != proof["body"]["accepted"]
                || actual["head"] != proof["body"]["selected_head"]
            {
                return Err(invalid());
            }
        }
        let mut body = self.current.body().clone();
        let generation = number(&body["committed_generation"])?
            .checked_add(1)
            .ok_or_else(invalid)?
            .to_string();
        let active = record(
            &rid(id, "active"),
            "ActiveSelection",
            json!({"device_id":body["device_id"],"workspace_slot_id":body["workspace_slot_id"],"committed_generation":generation,"slot_scope_sha256":body["slot_scope_sha256"],"activation_id":id,"target":target,"ready":rr}),
        );
        let ar = add(&mut body, active)?;
        let receipt = record(
            &rid(id, "receipt-Activated"),
            "ActivationReceipt",
            json!({"intent":intent_ref,"outcome":"Activated","old_committed_generation":body["committed_generation"],"new_committed_generation":generation,"source_saved":saved,"capsule":capsule,"target_ready":rr,"active":ar}),
        );
        add(&mut body, receipt)?;
        body["active"] = ar;
        body["pending"] = Value::Null;
        body["committed_generation"] = json!(generation);
        self.publish(body)?;
        self.session = opened;
        self.recovery = None;
        Ok(())
    }
    fn refuse(&mut self, id: &str) -> IoResult<()> {
        let (intent, proof) = self.pending(id)?;
        let body = &proof["body"];
        let saved = body["source_saved"].clone();
        let capsule = body["capsule"].clone();
        let ready = body["target_ready"].clone();
        if body["stage"] != "Refused" {
            self.progress(
                id,
                "Refused",
                saved.clone(),
                capsule.clone(),
                ready.clone(),
                vec![],
            )?;
        }
        if self.session.is_none() {
            self.restore();
        }
        let outcome = if self.recovery.is_some() && !capsule.is_null() {
            "RetainedReadOnlyRecovery"
        } else {
            "RetainedCurrent"
        };
        let mut body = self.current.body().clone();
        let receipt = record(
            &rid(id, &format!("receipt-{outcome}")),
            "ActivationReceipt",
            json!({"intent":v2::record_ref(&intent).map_err(bad)?,"outcome":outcome,"old_committed_generation":body["committed_generation"],"new_committed_generation":body["committed_generation"],"source_saved":saved,"capsule":capsule,"target_ready":ready,"active":body["active"]}),
        );
        add(&mut body, receipt)?;
        body["pending"] = Value::Null;
        self.publish(body)
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn confirm(&mut self, id: &str) -> Value {
        let result = self.confirm_inner(id);
        if let Err(evidence) = result {
            if self.sync().is_ok() {
                if self
                    .current
                    .receipt(id)
                    .is_some_and(|record| record["body"]["outcome"] == "Activated")
                {
                    self.restore();
                    if self.recovery.is_none() {
                        return self.state("Activated", Some(id), None);
                    }
                } else if self.current.pending().is_some() {
                    let _ = self.refuse(id);
                }
            }
            return self.state(self.outcome(id), Some(id), Some(evidence.to_string()));
        }
        self.state(self.outcome(id), Some(id), None)
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn cancel(&mut self, id: &str) -> Value {
        let result = self.sync().and_then(|()| {
            if self.current.receipt(id).is_some() {
                Ok(())
            } else {
                self.refuse(id)
            }
        });
        self.state(
            self.outcome(id),
            Some(id),
            result.err().map(|evidence| evidence.to_string()),
        )
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn retry(&mut self, id: &str) -> Value {
        let result = (|| {
            self.sync()?;
            if self.current.receipt(id).is_some() {
                if self.current.pending().is_none() && self.recovery.is_some() {
                    self.restore();
                }
                return Ok(());
            }
            self.pending(id)?;
            self.refuse(id)
        })();
        self.state(
            self.outcome(id),
            Some(id),
            result.err().map(|evidence| evidence.to_string()),
        )
    }
    #[expect(
        clippy::needless_pass_by_value,
        reason = "The facade transfers the bounded lifecycle request into this call."
    )]
    pub(in crate::package::v1_3::repeatable::disposable) fn lifecycle(
        &mut self,
        request: Value,
    ) -> Value {
        let result = (|| {
            self.sync()?;
            if self.closed || self.current.pending().is_some() {
                return Err(invalid());
            }
            let mut db = self.db.lock().map_err(bad)?;
            match request["kind"].as_str() {
                Some("create") => db.create(&request),
                Some("rename") => db.rename(&request),
                _ => Err(invalid()),
            }
        })();
        let mut state = self.state(
            if result.is_ok() { "Current" } else { "Refused" },
            None,
            result.as_ref().err().map(ToString::to_string),
        );
        if let Ok(receipt) = result {
            state["library_receipt"] = receipt;
        }
        state
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn library_command(
        &mut self,
        id: &str,
        library: &str,
        name: &str,
        revision: Option<u64>,
    ) -> Value {
        let registration = match self.db.lock() {
            Ok(db) => db.registration(),
            Err(evidence) => return self.state("Refused", None, Some(evidence.to_string())),
        };
        let mut request = json!({"schema":"photara.library-lifecycle.v1","kind":if revision.is_some(){"rename"}else{"create"},"authority":{"kind":"local","database_id":registration["database_id"],"epoch":registration["epoch"]},"principal":{"kind":"local","id":registration["principal_id"]},"operation_id":id,"initiating_device_id":registration["device_id"],"library_id":library,"name":name.trim()});
        if let Some(record) = revision {
            request["expected_revision"] = json!(record.to_string());
        }
        self.lifecycle(request)
    }
    pub(in crate::package::v1_3::repeatable::disposable) fn inject_fault(
        &mut self,
        point: &str,
    ) -> IoResult<()> {
        self.db.lock().map_err(bad)?.inject_fault(point)
    }
}
