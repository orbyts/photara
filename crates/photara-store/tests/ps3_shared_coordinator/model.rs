//! Disposable PS3 state model. Every lease/barrier/HEAD is injected memory state.
//! No production admission constructor, OS lock, transport, package codec or IO.

use photara_core::contracts::ids::OperationId;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell, collections::BTreeMap, collections::BTreeSet, collections::VecDeque, rc::Rc,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Coordinate {
    pub sequence: u64,
    pub value: i64,
    pub digest: [u8; 32],
}
impl Coordinate {
    pub fn new(sequence: u64, value: i64) -> Self {
        // Toy authored value, not a candidate Graph schema or permanent wire.
        let mut hash = Sha256::new();
        hash.update(sequence.to_be_bytes());
        hash.update(value.to_be_bytes());
        Self {
            sequence,
            value,
            digest: hash.finalize().into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub operation: OperationId,
    pub expected: Coordinate,
    pub value: i64,
}
impl Request {
    fn digest(&self) -> [u8; 32] {
        // Fixed typed fields in a fixture-only encoding; no production format.
        let mut hash = Sha256::new();
        hash.update(self.operation.uuid().as_bytes());
        hash.update(self.expected.sequence.to_be_bytes());
        hash.update(self.expected.value.to_be_bytes());
        hash.update(self.expected.digest);
        hash.update(self.value.to_be_bytes());
        hash.finalize().into()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Surface {
    Gui,
    Headless,
    Agent,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Attachment {
    epoch: u64,
    slot: u8,
    generation: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Grant {
    pub principal: u8,
    pub generation: u64,
    pub incarnation: u64,
    pub active: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Run {
    Idle,
    Running,
    StopRequested,
}
#[derive(Clone, Copy, Debug)]
struct Attached {
    token: Attachment,
    surface: Surface,
    grant: Grant,
    frozen: bool,
    run: Run,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Accepted {
    pub request: Request,
    pub request_digest: [u8; 32],
    pub principal: u8,
    pub grant_generation: u64,
    pub after: Coordinate,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Saved {
    pub covered: Coordinate,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlushTicket {
    attachment: Attachment,
    target: Coordinate,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Submission {
    Queued,
    Existing(Accepted),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    WriterBusy,
    OldOwner,
    OldAttachment,
    Unauthorized,
    Scope,
    Frozen,
    Backpressure,
    IntentConflict,
    Stale,
    Empty,
    JournalFailed,
    OutcomeUnknown,
    StorageConflict,
    CorruptJournal,
    FlushFailed,
    TooLate,
    RunActive,
    Pending,
    Unsaved,
    Exiting,
    Overflow,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Saved,
    Saving,
    SaveFailed,
    RecoveryRequired,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalFault {
    None,
    BeforeBarrier,
    AfterBarrier,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlushFault {
    None,
    BeforeReplace,
    AfterReplace,
    AfterReceipt,
}

#[derive(Clone, Copy, Debug)]
struct Publication {
    before: Coordinate,
    target: Coordinate,
}

/// A synthetic registrar owns grants and a cooperative lease. Crash persistence
/// is represented by vectors surviving replacement of the Coordinator object.
pub struct FakeStorage {
    owner: Option<u64>,
    next_epoch: u64,
    pub incarnation: u64,
    pub grants: BTreeMap<u8, Grant>,
    pub journal: Vec<Accepted>,
    pub head: Coordinate,
    verified: Coordinate,
    publication: Option<Publication>,
    pub replacements: usize,
    pub journal_barriers: usize,
}
pub type Storage = Rc<RefCell<FakeStorage>>;
impl FakeStorage {
    pub fn new() -> Storage {
        Rc::new(RefCell::new(Self {
            owner: None,
            next_epoch: 0,
            incarnation: 41,
            grants: (1..=3)
                .map(|principal| {
                    (
                        principal,
                        Grant {
                            principal,
                            generation: 1,
                            incarnation: 41,
                            active: true,
                        },
                    )
                })
                .collect(),
            journal: Vec::new(),
            head: Coordinate::new(0, 0),
            verified: Coordinate::new(0, 0),
            publication: None,
            replacements: 0,
            journal_barriers: 0,
        }))
    }
    pub fn crash_owner(&mut self) {
        self.owner = None;
    }
    fn validate(&self, epoch: u64) -> Result<(), Refusal> {
        if self.owner != Some(epoch) {
            return Err(Refusal::OldOwner);
        }
        if self.incarnation != 41 {
            return Err(Refusal::StorageConflict);
        }
        let expected = self.publication.map_or(self.verified, |p| p.target);
        let old = self.publication.map_or(self.verified, |p| p.before);
        if (self.head != expected && self.head != old)
            || self.head.sequence < self.verified.sequence
        {
            return Err(Refusal::StorageConflict);
        }
        Ok(())
    }
    fn append(
        &mut self,
        epoch: u64,
        receipt: Accepted,
        fault: JournalFault,
    ) -> Result<(), Refusal> {
        self.validate(epoch)?;
        if fault == JournalFault::BeforeBarrier {
            return Err(Refusal::JournalFailed);
        }
        self.journal.push(receipt);
        self.journal_barriers += 1;
        if fault == JournalFault::AfterBarrier {
            return Err(Refusal::OutcomeUnknown);
        }
        Ok(())
    }
    fn flush(
        &mut self,
        epoch: u64,
        target: Coordinate,
        fault: FlushFault,
    ) -> Result<Saved, Refusal> {
        self.validate(epoch)?;
        if self.verified.sequence >= target.sequence {
            return Ok(Saved {
                covered: self.verified,
            });
        }
        if let Some(pending) = self.publication {
            // Resolve an earlier candidate with its original inclusion before
            // accepting a different candidate. No replacement ID or blind retry.
            if pending.target != target {
                return Err(Refusal::OutcomeUnknown);
            }
        } else {
            self.publication = Some(Publication {
                before: self.head,
                target,
            });
        }
        if fault == FlushFault::BeforeReplace {
            return Err(Refusal::FlushFailed);
        }
        if self.head != target {
            self.head = target;
            self.replacements += 1;
        }
        if fault == FlushFault::AfterReplace {
            return Err(Refusal::OutcomeUnknown);
        }
        // This assignment stands in for injected barriers, exact reopen, and
        // durable receipt. It asserts no actual hardware/storage guarantee.
        self.verified = target;
        self.publication = None;
        if fault == FlushFault::AfterReceipt {
            return Err(Refusal::OutcomeUnknown);
        }
        Ok(Saved { covered: target })
    }
}

#[derive(Clone)]
struct Queued {
    attachment: Attachment,
    request: Request,
}
pub struct Coordinator {
    storage: Storage,
    epoch: u64,
    attachments: BTreeMap<u8, Attached>,
    queue: VecDeque<Queued>,
    limit: usize,
    pub current: Coordinate,
    saved: Coordinate,
    accepted: Vec<Accepted>,
    blocked: Option<Refusal>,
    exiting: bool,
}
impl Coordinator {
    pub fn claim(storage: Storage, limit: usize) -> Result<Self, Refusal> {
        let epoch = {
            let mut state = storage.borrow_mut();
            if state.owner.is_some() {
                return Err(Refusal::WriterBusy);
            }
            state.next_epoch = state.next_epoch.checked_add(1).ok_or(Refusal::Overflow)?;
            let epoch = state.next_epoch;
            state.owner = Some(epoch);
            epoch
        };
        let mut this = Self {
            storage,
            epoch,
            attachments: BTreeMap::new(),
            queue: VecDeque::new(),
            limit,
            current: Coordinate::new(0, 0),
            saved: Coordinate::new(0, 0),
            accepted: Vec::new(),
            blocked: None,
            exiting: false,
        };
        if let Err(error) = this.recover() {
            // Retain the lease in read-only recovery, never admit another owner
            // merely because validation failed.
            this.blocked = Some(error);
        }
        Ok(this)
    }
    pub fn recover(&mut self) -> Result<(), Refusal> {
        let result = self.recover_inner();
        self.blocked = result.as_ref().err().copied();
        result
    }
    fn recover_inner(&mut self) -> Result<(), Refusal> {
        let state = self.storage.borrow();
        state.validate(self.epoch)?;
        let mut before = Coordinate::new(0, 0);
        let mut operations = BTreeSet::new();
        for receipt in &state.journal {
            let sequence = before.sequence.checked_add(1).ok_or(Refusal::Overflow)?;
            if receipt.request.expected != before
                || receipt.request_digest != receipt.request.digest()
                || receipt.after != Coordinate::new(sequence, receipt.request.value)
                || !operations.insert(receipt.request.operation)
            {
                return Err(Refusal::CorruptJournal);
            }
            before = receipt.after;
        }
        let coordinate_exists = |value: Coordinate| {
            value == Coordinate::new(0, 0)
                || state.journal.iter().any(|receipt| receipt.after == value)
        };
        if !coordinate_exists(state.verified) || !coordinate_exists(state.head) {
            return Err(Refusal::StorageConflict);
        }
        self.current = before;
        self.saved = state.verified;
        self.accepted.clone_from(&state.journal);
        Ok(())
    }
    pub fn attach(
        &mut self,
        slot: u8,
        surface: Surface,
        principal: u8,
    ) -> Result<Attachment, Refusal> {
        self.storage.borrow().validate(self.epoch)?;
        if self.exiting {
            return Err(Refusal::Exiting);
        }
        let grant = *self
            .storage
            .borrow()
            .grants
            .get(&principal)
            .ok_or(Refusal::Unauthorized)?;
        if !grant.active {
            return Err(Refusal::Unauthorized);
        }
        if grant.incarnation != 41 {
            return Err(Refusal::Scope);
        }
        let generation = self.attachments.get(&slot).map_or(Ok(1), |old| {
            old.token.generation.checked_add(1).ok_or(Refusal::Overflow)
        })?;
        let token = Attachment {
            epoch: self.epoch,
            slot,
            generation,
        };
        self.attachments.insert(
            slot,
            Attached {
                token,
                surface,
                grant,
                frozen: false,
                run: Run::Idle,
            },
        );
        Ok(token)
    }
    fn authorize(&self, token: Attachment) -> Result<Attached, Refusal> {
        self.storage.borrow().validate(self.epoch)?;
        if token.epoch != self.epoch {
            return Err(Refusal::OldOwner);
        }
        let attached = *self
            .attachments
            .get(&token.slot)
            .ok_or(Refusal::OldAttachment)?;
        if attached.token != token {
            return Err(Refusal::OldAttachment);
        }
        let storage = self.storage.borrow();
        let current = storage
            .grants
            .get(&attached.grant.principal)
            .ok_or(Refusal::Unauthorized)?;
        if !current.active || current.generation != attached.grant.generation {
            return Err(Refusal::Unauthorized);
        }
        if current.incarnation != 41 {
            return Err(Refusal::Scope);
        }
        Ok(attached)
    }
    pub fn surface(&self, token: Attachment) -> Result<Surface, Refusal> {
        Ok(self.authorize(token)?.surface)
    }
    fn prior(&self, principal: u8, request: &Request) -> Result<Option<Accepted>, Refusal> {
        let Some(receipt) = self
            .accepted
            .iter()
            .find(|r| r.request.operation == request.operation)
        else {
            return Ok(None);
        };
        if receipt.principal != principal {
            return Err(Refusal::Unauthorized);
        }
        if receipt.request_digest != request.digest() {
            return Err(Refusal::IntentConflict);
        }
        Ok(Some(receipt.clone()))
    }
    pub fn submit(&mut self, token: Attachment, request: Request) -> Result<Submission, Refusal> {
        let attached = self.authorize(token)?;
        if let Some(error) = self.blocked {
            return Err(error);
        }
        // Authorized duplicate lookup precedes stale expected-coordinate rejection.
        if let Some(receipt) = self.prior(attached.grant.principal, &request)? {
            return Ok(Submission::Existing(receipt));
        }
        if self.exiting {
            return Err(Refusal::Exiting);
        }
        if attached.frozen {
            return Err(Refusal::Frozen);
        }
        if let Some(old) = self
            .queue
            .iter()
            .find(|q| q.request.operation == request.operation)
        {
            let old_actor = self.authorize(old.attachment)?;
            if old_actor.grant.principal != attached.grant.principal {
                return Err(Refusal::Unauthorized);
            }
            if old.request != request {
                return Err(Refusal::IntentConflict);
            }
            return Ok(Submission::Queued);
        }
        if self.queue.len() >= self.limit {
            return Err(Refusal::Backpressure);
        }
        self.queue.push_back(Queued {
            attachment: token,
            request,
        });
        Ok(Submission::Queued)
    }
    pub fn process_next(&mut self, fault: JournalFault) -> Result<Accepted, Refusal> {
        self.storage.borrow().validate(self.epoch)?;
        if let Some(error) = self.blocked {
            return Err(error);
        }
        let queued = self.queue.front().cloned().ok_or(Refusal::Empty)?;
        let attached = match self.authorize(queued.attachment) {
            Ok(value) => value,
            Err(error) => {
                self.queue.pop_front();
                return Err(error);
            }
        };
        if let Some(receipt) = self.prior(attached.grant.principal, &queued.request)? {
            self.queue.pop_front();
            return Ok(receipt);
        }
        if queued.request.expected != self.current {
            self.queue.pop_front();
            return Err(Refusal::Stale);
        }
        let sequence = self
            .current
            .sequence
            .checked_add(1)
            .ok_or(Refusal::Overflow)?;
        let receipt = Accepted {
            request_digest: queued.request.digest(),
            principal: attached.grant.principal,
            grant_generation: attached.grant.generation,
            after: Coordinate::new(sequence, queued.request.value),
            request: queued.request,
        };
        if let Err(error) = self
            .storage
            .borrow_mut()
            .append(self.epoch, receipt.clone(), fault)
        {
            self.blocked = Some(error);
            return Err(error); // Pending client input stays in the queue.
        }
        self.queue.pop_front();
        self.current = receipt.after;
        self.accepted.push(receipt.clone());
        Ok(receipt)
    }
    pub fn cancel_queued(
        &mut self,
        token: Attachment,
        operation: OperationId,
    ) -> Result<(), Refusal> {
        let attached = self.authorize(token)?;
        if let Some(error) = self.blocked {
            return Err(error);
        }
        if let Some(receipt) = self
            .accepted
            .iter()
            .find(|r| r.request.operation == operation)
        {
            if receipt.principal != attached.grant.principal {
                return Err(Refusal::Unauthorized);
            }
            return Err(Refusal::TooLate);
        }
        let index = self
            .queue
            .iter()
            .position(|q| q.request.operation == operation)
            .ok_or(Refusal::Empty)?;
        if self.queue[index].attachment != token {
            return Err(Refusal::Unauthorized);
        }
        self.queue.remove(index);
        Ok(())
    }
    pub fn begin_flush(&self, token: Attachment) -> Result<FlushTicket, Refusal> {
        self.authorize(token)?;
        if let Some(error) = self.blocked {
            return Err(error);
        }
        if self.queue.iter().any(|q| q.attachment == token) {
            return Err(Refusal::Pending);
        }
        Ok(FlushTicket {
            attachment: token,
            target: self.current,
        })
    }
    pub fn complete_flush(
        &mut self,
        ticket: FlushTicket,
        fault: FlushFault,
    ) -> Result<Saved, Refusal> {
        self.authorize(ticket.attachment)?;
        if let Some(error) = self.blocked {
            return Err(error);
        }
        let result = self
            .storage
            .borrow_mut()
            .flush(self.epoch, ticket.target, fault);
        match result {
            Ok(receipt) => {
                self.saved = receipt.covered;
                Ok(receipt)
            }
            Err(error) => {
                self.blocked = Some(error);
                Err(error)
            }
        }
    }
    pub fn freeze(&mut self, token: Attachment) -> Result<(), Refusal> {
        let attached = self.authorize(token)?;
        if attached.run != Run::Idle {
            return Err(Refusal::RunActive);
        }
        self.attachments.get_mut(&token.slot).unwrap().frozen = true;
        Ok(())
    }
    pub fn thaw(&mut self, token: Attachment) -> Result<(), Refusal> {
        self.authorize(token)?;
        if self.exiting {
            return Err(Refusal::Exiting);
        }
        if let Some(error) = self.blocked {
            return Err(error);
        }
        self.attachments.get_mut(&token.slot).unwrap().frozen = false;
        Ok(())
    }
    pub fn start_run(&mut self, token: Attachment) -> Result<(), Refusal> {
        let attached = self.authorize(token)?;
        if attached.frozen || self.exiting {
            return Err(Refusal::Frozen);
        }
        self.attachments.get_mut(&token.slot).unwrap().run = Run::Running;
        Ok(())
    }
    pub fn request_stop(&mut self, token: Attachment) -> Result<(), Refusal> {
        self.authorize(token)?;
        self.attachments.get_mut(&token.slot).unwrap().run = Run::StopRequested;
        Ok(())
    }
    pub fn run_settled(&mut self, token: Attachment) -> Result<(), Refusal> {
        self.authorize(token)?;
        // Injected terminal evidence includes settled callbacks/effect bookkeeping.
        self.attachments.get_mut(&token.slot).unwrap().run = Run::Idle;
        Ok(())
    }
    pub fn request_exit(&mut self) -> Result<(), Refusal> {
        self.storage.borrow().validate(self.epoch)?;
        if self.attachments.values().any(|a| a.run != Run::Idle) {
            return Err(Refusal::RunActive);
        }
        self.exiting = true;
        for attached in self.attachments.values_mut() {
            attached.frozen = true;
        }
        Ok(())
    }
    pub fn finish_exit(&mut self) -> Result<(), Refusal> {
        self.storage.borrow().validate(self.epoch)?;
        if !self.exiting {
            return Err(Refusal::Pending);
        }
        if !self.queue.is_empty() {
            return Err(Refusal::Pending);
        }
        if self.blocked.is_some() || self.saved != self.current {
            return Err(Refusal::Unsaved);
        }
        self.storage.borrow_mut().owner = None;
        Ok(())
    }
    pub fn status(&self) -> Status {
        if self.storage.borrow().validate(self.epoch).is_err() {
            return Status::RecoveryRequired;
        }
        match self.blocked {
            Some(Refusal::JournalFailed | Refusal::FlushFailed) => Status::SaveFailed,
            Some(_) => Status::RecoveryRequired,
            None if self.current == self.saved && self.queue.is_empty() => Status::Saved,
            None => Status::Saving,
        }
    }
    pub fn queued(&self) -> usize {
        self.queue.len()
    }
}
