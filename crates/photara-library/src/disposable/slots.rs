use super::{
    Connection, Deserialize, LocalDatabase, OptionalExtension, Registration, Result,
    SelectionStore, Serialize, Snapshot, TransactionBehavior, Uuid, Value, canonical, decimal,
    digest, ensure, err, fields, hex, json, lifecycle, params, sha, sql, uuid, v2,
};
type Projection = (Option<Vec<u8>>, Option<Vec<u8>>);
type SlotRow = (
    Vec<u8>,
    i64,
    i64,
    i64,
    Vec<u8>,
    Vec<u8>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
);
use photara_core::contracts::access::{ActionMask, ProjectAction};
fn context_value(reg: &Registration, library: Uuid) -> Value {
    json!({"authority":reg.authority(),"principal":reg.principal(),"library_id":library})
}
fn checked_context(conn: &Connection, reg: &Registration, target: &Value) -> Result<Value> {
    let kind = target["kind"].as_str().ok_or_else(|| err("target kind"))?;
    fields(
        target,
        if kind == "library" {
            &["kind", "context"]
        } else {
            &["kind", "context", "project"]
        },
    )?;
    ensure(matches!(kind, "library" | "project"))?;
    let context = &target["context"];
    fields(context, &["authority", "principal", "library_id"])?;
    ensure(context["authority"] == reg.authority() && context["principal"] == reg.principal())?;
    let library = uuid(&context["library_id"])?;
    lifecycle::local(conn, reg, library)?;
    let (revision,contract,generation):(i64,i64,i64)=conn.query_row("SELECT l.local_revision,c.local_revision,c.authorization_generation FROM libraries l JOIN library_contract_state c USING(library_id) WHERE l.library_id=?",[library.as_bytes()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(sql)?;
    if kind == "project" {
        let project = &target["project"];
        fields(
            project,
            &[
                "library_id",
                "project_id",
                "incarnation_id",
                "graph_id",
                "registration_sha256",
            ],
        )?;
        ensure(uuid(&project["library_id"])? == library)?;
        uuid(&project["incarnation_id"])?;
        uuid(&project["graph_id"])?;
        digest(
            project["registration_sha256"]
                .as_str()
                .ok_or_else(|| err("registration digest"))?,
        )?;
        let project = uuid(&project["project_id"])?;
        let bits: Option<i64> = conn
            .query_row(
                crate::gen2::PROJECT_AUTHORITY_SQL,
                params![
                    library.as_bytes(),
                    project.as_bytes(),
                    reg.principal_id.as_bytes()
                ],
                |r| r.get(0),
            )
            .optional()
            .map_err(sql)?;
        let mask = bits
            .and_then(|v| u16::try_from(v).ok())
            .and_then(|v| ActionMask::new(v).ok())
            .ok_or_else(|| err("Project access denied"))?;
        ensure(mask.allows(ProjectAction::Read) && bits.is_some_and(|b| b & 0x47 == 0x47))?;
        let policy:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM project_access_policies WHERE library_id=? AND project_id=? AND record_schema=1)",params![library.as_bytes(),project.as_bytes()],|r|r.get(0)).map_err(sql)?;
        ensure(policy)?;
    }
    Ok(
        json!({"context":context,"library_revision":revision.to_string(),"contract_revision":contract.to_string(),"authorization_generation":generation.to_string()}),
    )
}
fn projection(snapshot: &Snapshot) -> Result<Projection> {
    let Some(active) = snapshot.active() else {
        return Ok((None, None));
    };
    let record = snapshot.record(active).map_err(|e| err(&e.to_string()))?;
    let target = &record["body"]["target"];
    let library = uuid(&target["context"]["library_id"])?;
    let project = if target["kind"] == "project" {
        Some(uuid(&target["project"]["project_id"])?.as_bytes().to_vec())
    } else {
        None
    };
    Ok((Some(library.as_bytes().to_vec()), project))
}
fn read_slot(conn: &Connection, reg: &Registration) -> Result<Snapshot> {
    let (scope,revision,request,committed,bytes,hash,library,project):SlotRow=conn.query_row("SELECT slot_scope_sha256,revision,request_generation,committed_generation,snapshot_canonical,snapshot_sha256,active_library_id,active_project_id FROM local_activation_slots WHERE device_id=? AND workspace_slot_id=?",params![reg.device_id.as_bytes(),reg.workspace_slot_id.as_bytes()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?))).map_err(sql)?;
    ensure(scope == digest(&reg.slot_scope_sha256)? && hash == sha(&bytes))?;
    let snapshot = Snapshot::parse(&bytes, reg.limits()).map_err(|e| err(&e.to_string()))?;
    let body = snapshot.body();
    ensure(
        body["device_id"] == reg.device_id.to_string()
            && body["workspace_slot_id"] == reg.workspace_slot_id.to_string()
            && body["slot_scope_sha256"] == reg.slot_scope_sha256
            && decimal(&body["revision"])? == revision
            && decimal(&body["request_generation"])? == request
            && decimal(&body["committed_generation"])? == committed
            && projection(&snapshot)? == (library, project),
    )?;
    Ok(snapshot)
}
/// Registrar supplies an independently verified package association. This API
/// never opens package paths or substitutes a stored scalar for native evidence.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectRegistration {
    pub library_id: Uuid,
    pub project_id: Uuid,
    pub association_commit_id: Uuid,
    pub association_sha256: String,
    pub grant_id: Uuid,
}
impl LocalDatabase {
    pub(super) fn load_checked(&self) -> Result<Snapshot> {
        read_slot(&self.connection, &self.registration)
    }
    /// Registers a fresh disposable Project using the existing restricted-policy,
    /// local-controller manager grant semantics. No overwrite or adoption path.
    /// # Errors
    /// Refuses stale authority, duplicate identity or invalid association.
    pub fn register_project(&mut self, p: &ProjectRegistration, at: i64) -> Result<()> {
        self.check_pins()?;
        self.settings()?;
        self.capacity()?;
        ensure(
            at >= 0
                && ![
                    p.library_id,
                    p.project_id,
                    p.association_commit_id,
                    p.grant_id,
                ]
                .iter()
                .any(Uuid::is_nil),
        )?;
        let hash = digest(&p.association_sha256)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql)?;
        lifecycle::local(&tx, &self.registration, p.library_id)?;
        tx.execute(
            "INSERT INTO project_ownership VALUES(?,?,'active',?,?,NULL,'sealed-v1',1,1,?,?)",
            params![
                p.project_id.as_bytes(),
                p.library_id.as_bytes(),
                p.association_commit_id.as_bytes(),
                hash,
                at,
                at
            ],
        )
        .map_err(sql)?;
        tx.execute(
            "INSERT INTO project_access_policies VALUES(?,?,'restricted',0,0,0,0,1,1,1,?,?)",
            params![p.library_id.as_bytes(), p.project_id.as_bytes(), at, at],
        )
        .map_err(sql)?;
        tx.execute(
            "INSERT INTO project_access_grants VALUES(?,?,?,NULL,?,255,'active',NULL,1,1,?,?)",
            params![
                p.grant_id.as_bytes(),
                p.library_id.as_bytes(),
                p.project_id.as_bytes(),
                self.registration.principal_id.as_bytes(),
                at,
                at
            ],
        )
        .map_err(sql)?;
        tx.execute("INSERT INTO project_catalog(library_id,project_id,visibility,local_revision,created_at_ms,updated_at_ms) VALUES(?,?,'visible',1,?,?)",params![p.library_id.as_bytes(),p.project_id.as_bytes(),at,at]).map_err(sql)?;
        tx.execute("UPDATE library_contract_state SET authorization_generation=authorization_generation+1,local_revision=local_revision+1,updated_at=? WHERE library_id=?",params![at,p.library_id.as_bytes()]).map_err(sql)?;
        tx.commit().map_err(sql)?;
        self.capacity()
    }
}
impl SelectionStore for LocalDatabase {
    fn inject_fault(&mut self, point: &str) -> Result<()> {
        LocalDatabase::inject_fault(self, point)
    }
    fn registration(&self) -> Value {
        self.registration.scope()
    }
    fn load(&self) -> Result<Vec<u8>> {
        self.check_pins()?;
        self.settings()?;
        let tx = self.connection.unchecked_transaction().map_err(sql)?;
        let snapshot = read_slot(&tx, &self.registration)?;
        tx.commit().map_err(sql)?;
        self.barriers()?;
        self.check_pins()?;
        Ok(snapshot.bytes().to_vec())
    }
    fn publish(&mut self, expected_sha256: &str, candidate: &[u8]) -> Result<()> {
        self.check_pins()?;
        self.settings()?;
        self.capacity()?;
        digest(expected_sha256)?;
        let next = Snapshot::parse(candidate, self.registration.limits())
            .map_err(|e| err(&e.to_string()))?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql)?;
        let old = read_slot(&tx, &self.registration)?;
        if old.bytes() == candidate {
            ensure(old.sha256() == hex(&sha(candidate)))?;
            if let Some(active) = old.active() {
                let record = old.record(active).map_err(|e| err(&e.to_string()))?;
                checked_context(&tx, &self.registration, &record["body"]["target"])?;
            }
            tx.commit().map_err(sql)?;
            self.barriers()?;
            return Ok(());
        }
        let transition = v2::validate_transition(
            &old,
            &next,
            expected_sha256,
            &self.registration.slot_scope_sha256,
        )
        .map_err(|e| err(&e.to_string()))?;
        // Current selection is live-checked even when immutable stored proof exists.
        if let Some(active) = next.active() {
            let record = next.record(active).map_err(|e| err(&e.to_string()))?;
            checked_context(&tx, &self.registration, &record["body"]["target"])?;
        }
        if let Some(pending) = next.pending() {
            let progress = next.record(pending).map_err(|e| err(&e.to_string()))?;
            let intent = next
                .record(&progress["body"]["intent"])
                .map_err(|e| err(&e.to_string()))?;
            if progress["body"]["stage"] != "Refused" {
                let actual = checked_context(&tx, &self.registration, &intent["body"]["target"])?;
                if progress["body"]["stage"] == "TargetReady" {
                    let ready = next
                        .record(&progress["body"]["target_ready"])
                        .map_err(|e| err(&e.to_string()))?;
                    let stored = next
                        .record(&ready["body"]["context_proof"])
                        .map_err(|e| err(&e.to_string()))?;
                    ensure(stored["body"] == actual)?;
                }
            }
        }
        if next.active() != old.active() {
            let active = next
                .record(next.active().ok_or_else(|| err("active expected"))?)
                .map_err(|e| err(&e.to_string()))?;
            let ready = next
                .record(&active["body"]["ready"])
                .map_err(|e| err(&e.to_string()))?;
            let stored = next
                .record(&ready["body"]["context_proof"])
                .map_err(|e| err(&e.to_string()))?;
            ensure(
                stored["body"]
                    == checked_context(&tx, &self.registration, &active["body"]["target"])?,
            )?;
        }
        let body = next.body();
        let (library, project) = projection(&next)?;
        ensure(tx.execute("UPDATE local_activation_slots SET revision=?,request_generation=?,committed_generation=?,snapshot_canonical=?,snapshot_sha256=?,active_library_id=?,active_project_id=? WHERE device_id=? AND workspace_slot_id=? AND slot_scope_sha256=? AND revision=? AND snapshot_sha256=?",params![decimal(&body["revision"])?,decimal(&body["request_generation"])?,decimal(&body["committed_generation"])?,candidate,sha(candidate),library,project,self.registration.device_id.as_bytes(),self.registration.workspace_slot_id.as_bytes(),digest(&self.registration.slot_scope_sha256)?,decimal(&old.body()["revision"])?,digest(expected_sha256)?]).map_err(sql)?==1)?;
        Self::trip(&mut self.fault, "before-commit")?;
        if transition == v2::Transition::Activated {
            Self::trip(&mut self.fault, "before-activation-commit")?;
        }
        tx.commit().map_err(sql)?;
        Self::trip(&mut self.fault, "after-commit")?;
        if transition == v2::Transition::Activated {
            Self::trip(&mut self.fault, "after-activation-commit")?;
        }
        self.check_pins()?;
        self.capacity()?;
        ensure(self.load_checked()?.bytes() == candidate)?;
        Ok(())
    }
    fn context(&self, target: &Value) -> Result<Value> {
        self.check_pins()?;
        self.settings()?;
        let tx = self.connection.unchecked_transaction().map_err(sql)?;
        let proof = checked_context(&tx, &self.registration, target)?;
        tx.commit().map_err(sql)?;
        Ok(proof)
    }
    fn libraries(&self) -> Result<Vec<Value>> {
        self.check_pins()?;
        let mut query=self.connection.prepare("SELECT l.library_id,l.display_name,l.local_revision FROM libraries l JOIN library_contract_state c USING(library_id) WHERE l.state='active' AND c.authority_mode='local-only' AND c.local_principal_id=? ORDER BY l.library_id").map_err(sql)?;
        let rows = query
            .query_map([self.registration.principal_id.as_bytes()], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(sql)?;
        let mut result = vec![];
        for row in rows {
            ensure(result.len() < self.registration.max_records)?;
            let (id, name, revision) = row.map_err(sql)?;
            let id = Uuid::from_slice(&id).map_err(|_| err("stored Library UUID"))?;
            result.push(json!({"id":id,"name":name,"context":context_value(&self.registration,id),"revision":revision.to_string()}));
        }
        Ok(result)
    }
    fn create(&mut self, request: &Value) -> Result<Value> {
        self.lifecycle(request, "create")
    }
    fn rename(&mut self, request: &Value) -> Result<Value> {
        self.lifecycle(request, "rename")
    }
    fn provenance(&self, target: &Value) -> Result<Value> {
        self.check_pins()?;
        self.settings()?;
        let tx = self.connection.unchecked_transaction().map_err(sql)?;
        let context = checked_context(&tx, &self.registration, target)?;
        ensure(target["kind"] == "project")?;
        let library = uuid(&target["context"]["library_id"])?;
        let project = uuid(&target["project"]["project_id"])?;
        let (grant,mask,grantrev,policyrev,policygen):(Vec<u8>,i64,i64,i64,i64)=tx.query_row("SELECT g.grant_id,g.action_mask,g.local_revision,p.local_revision,p.authorization_generation FROM project_access_grants g JOIN project_access_policies p USING(library_id,project_id) WHERE g.library_id=? AND g.project_id=? AND g.local_principal_id=? AND g.state='active'",params![library.as_bytes(),project.as_bytes(),self.registration.principal_id.as_bytes()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(sql)?;
        let grant = Uuid::from_slice(&grant).map_err(|_| err("stored grant UUID"))?;
        let observation = json!({"context":context,"grant_id":grant,"grant_mask":mask,"grant_revision":grantrev.to_string(),"policy_revision":policyrev.to_string(),"policy_generation":policygen.to_string()});
        let principal =
            json!({"kind":"local-controller","principal_id":self.registration.principal_id});
        let value = json!({"principal":principal,"grantor":principal,"actor":{"kind":"photara.gui","actor_id":"50000000-0000-4000-8000-000000000051"},"grant_ref":{"kind":"photara.project-grant","reference_id":grant},"effective_scope":{"project_id":project,"actions":71},"policy_decision_sha256":hex(&sha(&canonical(&observation)?))});
        tx.commit().map_err(sql)?;
        Ok(value)
    }
}
