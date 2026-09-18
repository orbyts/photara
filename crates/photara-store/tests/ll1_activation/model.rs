//! Disposable activation model: all evidence, leases and restart state are memory.
//! No SQL, filesystem, wire codec, UI or production capability constructor.

use photara_core::contracts::ids::{CommitId, GraphId, LibraryId, OperationId, ProjectId};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scope {
    pub authority: u64,
    pub principal: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Identity {
    pub library: LibraryId,
    pub project: ProjectId,
    pub incarnation: u64,
    pub bootstrap: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Saved {
    pub identity: Identity,
    pub owner: (u64, u64),
    pub through: u64,
    pub authored_revision: u64,
    pub authored_digest: [u8; 32],
    pub graph: (GraphId, u64, [u8; 32]),
    pub commit: CommitId,
    pub commit_digest: [u8; 32],
    pub head_digest: [u8; 32],
    pub package_revision: u64,
    pub capability: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Target {
    pub scope: Scope,
    pub library: LibraryId,
    pub project: Option<Identity>,
    pub graph: Option<GraphId>,
    pub view_version: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub operation: OperationId,
    pub target: Target,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetEvidence {
    pub target: Target,
    pub saved: Option<Saved>,
    pub restored: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub request: Request,
    pub generation: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    Busy,
    Stale,
    Evidence,
    RunActive,
    SaveUnknown,
    CapsuleFailed,
    RestoreFailed,
    PointerFailed,
    ReplyLost,
    IntentConflict,
    Fenced,
    NotReady,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Confirmation,
    Frozen,
    Detached,
}
#[derive(Clone, Debug)]
struct Intent {
    request: Request,
    slot_generation: u64,
    authorization_generation: u64,
    attachment_generation: u64,
    owner: (u64, u64),
    source: Target,
    stage: Stage,
    barrier: Option<Saved>,
    frozen_through: Option<u64>,
}
#[derive(Clone, Debug)]
pub struct Capsule {
    pub source: Target,
    pub barrier: Saved,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Recovery {
    Current(Target),
    Target(Target),
    ReadOnly,
    Fenced,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishFault {
    None,
    BeforeCommit,
    LostReply,
}
/// These vectors stand in for committed local records, NOT real durability.
pub struct Disk {
    pub visible: Option<Target>,
    pub generation: u64,
    pub receipts: BTreeMap<OperationId, Receipt>,
    pub capsules: BTreeMap<OperationId, Capsule>,
    intents: BTreeMap<OperationId, Intent>,
    pub capsule_writes: usize,
    pub removed_libraries: Vec<LibraryId>,
    pub file_effects: Vec<&'static str>,
    pub preferences: Vec<Target>,
}
pub type Storage = Rc<RefCell<Disk>>;
impl Disk {
    pub fn new(current: Target) -> Storage {
        Rc::new(RefCell::new(Self {
            visible: Some(current.clone()),
            generation: 1,
            receipts: BTreeMap::new(),
            capsules: BTreeMap::new(),
            intents: BTreeMap::new(),
            capsule_writes: 0,
            removed_libraries: vec![],
            file_effects: vec![],
            preferences: vec![current],
        }))
    }
}

// Independent injected host observations, not production coordinator state fields.
#[allow(clippy::struct_excessive_bools)]
pub struct Coordinator {
    pub disk: Storage,
    discovery: Option<Request>,
    pub source_ledger: BTreeMap<u64, Saved>,
    pub target_evidence: Option<TargetEvidence>,
    pub accepted_sequence: u64,
    pub owner: (u64, u64),
    pub attachment_generation: u64,
    pub authorization_generation: u64,
    pub gui_run_active: bool,
    pub stop_requested: bool,
    pub agent_run_active: bool,
    pub source_editable: bool,
    pub target_editable: bool,
    pub owner_exits: usize,
    pub grant_valid: bool,
}
impl Coordinator {
    pub fn new(disk: Storage, source: Saved, target: TargetEvidence) -> Self {
        Self {
            disk,
            discovery: None,
            accepted_sequence: source.through,
            owner: source.owner,
            source_ledger: [(source.through, source)].into(),
            target_evidence: Some(target),
            attachment_generation: 1,
            authorization_generation: 1,
            gui_run_active: false,
            stop_requested: false,
            agent_run_active: true,
            source_editable: true,
            target_editable: false,
            owner_exits: 0,
            grant_valid: true,
        }
    }
    pub fn discover(&mut self, request: Request) -> Result<(), Failure> {
        let disk = self.disk.borrow();
        if let Some(receipt) = disk.receipts.get(&request.operation) {
            return Err(if receipt.request == request {
                Failure::NotReady // Resolve committed outcome by lookup, never re-execute.
            } else {
                Failure::IntentConflict
            });
        }
        if disk.removed_libraries.contains(&request.target.library) {
            return Err(Failure::Fenced);
        }
        if !disk.intents.is_empty() {
            return Err(Failure::Busy);
        }
        self.discovery = Some(request);
        Ok(())
    }
    fn valid_saved(&self, saved: &Saved, through: u64, source: &Target) -> bool {
        source.project.as_ref() == Some(&saved.identity)
            && saved.identity.library == source.library
            && saved.owner == self.owner
            && saved.through == through
            && self.source_ledger.get(&through) == Some(saved)
    }
    pub fn prepare(&mut self, saved: Option<Saved>) -> Result<Option<OperationId>, Failure> {
        let request = self.discovery.clone().ok_or(Failure::NotReady)?;
        let mut disk = self.disk.borrow_mut();
        if !disk.intents.is_empty() {
            return Err(Failure::Busy);
        }
        let source = disk.visible.clone().ok_or(Failure::Fenced)?;
        if request.target.scope == source.scope
            && request.target.library == source.library
            && (request.target.project.is_none() || request.target == source)
        {
            self.discovery = None;
            return Ok(None); // Selecting current Library does not close its Project.
        }
        if self.gui_run_active {
            return Err(Failure::RunActive);
        }
        if !self.grant_valid {
            return Err(Failure::Fenced);
        }
        match (&source.project, &saved) {
            (Some(_), Some(s)) if self.valid_saved(s, self.accepted_sequence, &source) => {}
            (None, None) => {}
            _ => return Err(Failure::Evidence),
        }
        let operation = request.operation;
        let intent = Intent {
            request,
            source,
            slot_generation: disk.generation,
            authorization_generation: self.authorization_generation,
            attachment_generation: self.attachment_generation,
            owner: self.owner,
            stage: Stage::Confirmation,
            barrier: saved,
            frozen_through: None,
        };
        disk.intents.insert(operation, intent);
        self.discovery = None;
        self.source_editable = false; // The confirmation sheet blocks GUI mutation.
        Ok(Some(operation))
    }
    fn check(&self, intent: &Intent, disk: &Disk) -> Result<(), Failure> {
        if !self.grant_valid
            || disk.removed_libraries.contains(&intent.source.library)
            || disk
                .removed_libraries
                .contains(&intent.request.target.library)
        {
            return Err(Failure::Fenced);
        }
        if intent.slot_generation != disk.generation
            || intent.authorization_generation != self.authorization_generation
            || intent.attachment_generation != self.attachment_generation
            || intent.owner != self.owner
        {
            return Err(Failure::Stale);
        }
        Ok(())
    }
    pub fn confirmation_claim_current(&self, operation: OperationId) -> bool {
        self.disk.borrow().intents.get(&operation).is_some_and(|i| {
            i.barrier
                .as_ref()
                .is_some_and(|s| s.through == self.accepted_sequence)
        })
    }
    pub fn freeze(&mut self, operation: OperationId) -> Result<(), Failure> {
        let mut disk = self.disk.borrow_mut();
        let intent = disk.intents.get(&operation).ok_or(Failure::Stale)?;
        self.check(intent, &disk)?;
        if self.gui_run_active {
            return Err(Failure::RunActive);
        }
        if intent.stage != Stage::Confirmation {
            return Err(Failure::NotReady);
        }
        let intent = disk.intents.get_mut(&operation).unwrap();
        intent.frozen_through = intent
            .source
            .project
            .as_ref()
            .map(|_| self.accepted_sequence);
        intent.stage = Stage::Frozen;
        Ok(())
    }
    pub fn detach(
        &mut self,
        operation: OperationId,
        saved: Result<Option<Saved>, Failure>,
        capsule_succeeds: bool,
    ) -> Result<(), Failure> {
        let mut disk = self.disk.borrow_mut();
        let intent = disk.intents.get(&operation).ok_or(Failure::Stale)?;
        self.check(intent, &disk)?;
        if intent.stage != Stage::Frozen {
            return Err(Failure::NotReady);
        }
        let saved = saved?;
        match (intent.frozen_through, &saved) {
            (Some(n), Some(s)) if self.valid_saved(s, n, &intent.source) => {}
            (None, None) => {}
            _ => return Err(Failure::Evidence),
        }
        if let Some(barrier) = saved {
            if !capsule_succeeds {
                return Err(Failure::CapsuleFailed);
            }
            let capsule = Capsule {
                source: intent.source.clone(),
                barrier,
            };
            disk.file_effects.push("session:capsule-put");
            disk.capsule_writes += 1;
            disk.capsules.insert(operation, capsule);
        }
        disk.intents.get_mut(&operation).unwrap().stage = Stage::Detached;
        Ok(())
    }
    pub fn publish(
        &mut self,
        operation: OperationId,
        evidence: TargetEvidence,
        fault: PublishFault,
    ) -> Result<Receipt, Failure> {
        let mut disk = self.disk.borrow_mut();
        let intent = disk.intents.get(&operation).ok_or(Failure::Stale)?;
        self.check(intent, &disk)?;
        if intent.stage != Stage::Detached {
            return Err(Failure::NotReady);
        }
        if !evidence.restored
            || evidence.target != intent.request.target
            || self.target_evidence.as_ref() != Some(&evidence)
            || match (&evidence.target.project, &evidence.saved) {
                (Some(p), Some(s)) => {
                    p != &s.identity
                        || p.library != evidence.target.library
                        || evidence.target.graph != Some(s.graph.0)
                }
                (None, None) => evidence.target.graph.is_some(),
                _ => true,
            }
        {
            return Err(Failure::RestoreFailed);
        }
        if fault == PublishFault::BeforeCommit {
            return Err(Failure::PointerFailed);
        }
        let receipt = Receipt {
            request: intent.request.clone(),
            generation: disk.generation.checked_add(1).ok_or(Failure::Stale)?,
        };
        if disk.receipts.contains_key(&operation) {
            return Err(Failure::IntentConflict);
        }
        // One fake atomic publication, including scope + Project + Graph/view.
        disk.visible = Some(evidence.target);
        disk.preferences.push(receipt.request.target.clone());
        disk.generation = receipt.generation;
        disk.receipts.insert(operation, receipt.clone());
        disk.intents.remove(&operation);
        if fault == PublishFault::LostReply {
            return Err(Failure::ReplyLost);
        }
        Ok(receipt)
    }
    pub fn lookup(&self, request: &Request, scope: Scope) -> Result<Receipt, Failure> {
        if !self.grant_valid || request.target.scope != scope {
            return Err(Failure::Fenced);
        }
        let disk = self.disk.borrow();
        let receipt = disk
            .receipts
            .get(&request.operation)
            .ok_or(Failure::NotReady)?;
        if &receipt.request != request {
            return Err(Failure::IntentConflict);
        }
        Ok(receipt.clone())
    }
    pub fn establish(&mut self, operation: OperationId, succeeds: bool) -> Result<(), Failure> {
        let mut disk = self.disk.borrow_mut();
        let receipt = disk.receipts.get(&operation).ok_or(Failure::NotReady)?;
        if !self.grant_valid
            || disk.visible.as_ref() != Some(&receipt.request.target)
            || disk.generation != receipt.generation
        {
            return Err(Failure::Fenced);
        }
        if !succeeds {
            return Err(Failure::RestoreFailed);
        }
        self.target_editable = receipt.request.target.project.is_some();
        self.source_editable = false;
        if disk.capsules.remove(&operation).is_some() {
            disk.file_effects.push("session:capsule-release");
        }
        Ok(())
    }
    pub fn cancel(&mut self, operation: OperationId, reacquire: bool) -> Result<Recovery, Failure> {
        let mut disk = self.disk.borrow_mut();
        if disk.receipts.contains_key(&operation) {
            return Err(Failure::Stale);
        }
        let intent = disk.intents.get(&operation).ok_or(Failure::Stale)?;
        let authorized = self.check(intent, &disk).is_ok();
        let intent = disk.intents.remove(&operation).unwrap();
        self.source_editable = reacquire && authorized;
        Ok(if self.source_editable {
            Recovery::Current(intent.source)
        } else {
            Recovery::ReadOnly
        })
    }
    /// Restart oracle reads only the fake committed records; no real IO occurs.
    pub fn recover(disk: &Storage, operation: OperationId, reacquire: bool) -> Recovery {
        let disk = disk.borrow();
        if disk.visible.is_none() {
            return Recovery::Fenced;
        }
        if let Some(receipt) = disk.receipts.get(&operation) {
            if disk.visible.as_ref() != Some(&receipt.request.target)
                || disk.generation != receipt.generation
            {
                return Recovery::Fenced;
            }
            return if reacquire {
                Recovery::Target(receipt.request.target.clone())
            } else {
                Recovery::ReadOnly
            };
        }
        if let Some(capsule) = disk.capsules.get(&operation) {
            if disk.removed_libraries.contains(&capsule.source.library) {
                return Recovery::Fenced;
            }
            return if reacquire {
                Recovery::Current(capsule.source.clone())
            } else {
                Recovery::ReadOnly
            };
        }
        Recovery::Current(disk.visible.clone().unwrap())
    }
    pub fn remove_library(&mut self, library: LibraryId) {
        let mut disk = self.disk.borrow_mut();
        let affected = disk.visible.as_ref().is_some_and(|t| t.library == library)
            || disk
                .intents
                .values()
                .any(|i| i.source.library == library || i.request.target.library == library);
        disk.removed_libraries.push(library);
        disk.intents
            .retain(|_, i| i.source.library != library && i.request.target.library != library);
        if disk.visible.as_ref().is_some_and(|t| t.library == library) {
            disk.visible = None;
            disk.generation += 1;
        }
        if affected {
            self.source_editable = false;
            self.target_editable = false;
        }
        // Deliberately no capsule lookup, file adapter call, or package operation.
    }
}
