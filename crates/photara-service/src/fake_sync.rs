//! In-memory transport harness. It exercises online protocol transitions without activating the native `SQLite` worker.
use crate::{
    Actor, Batch, ContentReceipt, ContentRoot, Cursor, PublishContent, Request, Result, Service,
    ServiceError, Snapshot, canonical, hash,
};
use photara_core::contracts::{AccountId, OperationId};
use std::collections::BTreeMap;
use uuid::Uuid;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntentState {
    Sealed,
    Unknown,
    Acknowledged,
    Applied,
    Conflict,
}
#[derive(Clone, Debug)]
struct Intent {
    request: PublishContent,
    sealed: Vec<u8>,
    state: IntentState,
    receipt: Option<ContentReceipt>,
}
/// No persistence or real provider. Working overlays survive bootstrap and access loss.
pub struct FakeSync {
    actor: AccountId,
    request: Request,
    cursor: Option<Cursor>,
    base: BTreeMap<(&'static str, Uuid), ContentRoot>,
    intents: BTreeMap<OperationId, Intent>,
    access_lost: bool,
    v1_unsettled: bool,
}
impl FakeSync {
    #[must_use]
    pub fn new(actor: &Actor, request: Request) -> Self {
        Self {
            actor: actor.account(),
            request,
            cursor: None,
            base: BTreeMap::new(),
            intents: BTreeMap::new(),
            access_lost: false,
            v1_unsettled: false,
        }
    }
    pub fn set_v1_unsettled(&mut self, value: bool) {
        self.v1_unsettled = value;
    }
    #[must_use]
    pub const fn access_lost(&self) -> bool {
        self.access_lost
    }
    #[must_use]
    pub fn state(&self, operation: OperationId) -> Option<IntentState> {
        self.intents.get(&operation).map(|v| v.state)
    }
    #[must_use]
    pub fn roots(&self) -> Vec<ContentRoot> {
        self.base.values().cloned().collect()
    }
    /// # Errors
    /// Refuses changed requests under retained operation IDs or a foreign scope.
    pub fn seal(&mut self, c: PublishContent) -> Result<()> {
        c.root.validate(self.request.scope)?;
        let sealed = canonical(&c)?;
        if let Some(old) = self.intents.get(&c.operation) {
            return if old.sealed == sealed {
                Ok(())
            } else {
                Err(ServiceError::Conflict)
            };
        }
        self.intents.insert(
            c.operation,
            Intent {
                request: c,
                sealed,
                state: IntentState::Sealed,
                receipt: None,
            },
        );
        Ok(())
    }
    /// # Errors
    /// Requires exact actor/scope, settled v1 work and a fresh authorized bounded bootstrap.
    pub async fn bootstrap(
        &mut self,
        service: &Service,
        actor: &Actor,
        request: Request,
    ) -> Result<()> {
        if actor.account() != self.actor
            || request.scope != self.request.scope
            || request.device != self.request.device
            || self.v1_unsettled
        {
            return Err(ServiceError::Forbidden);
        }
        let snapshot = match service.snapshot(actor, request).await {
            Ok(v) => v,
            Err(ServiceError::Forbidden) => {
                self.access_lost = true;
                return Err(ServiceError::Forbidden);
            }
            Err(e) => return Err(e),
        };
        self.install(service, actor, request, snapshot)?;
        self.request = request;
        self.access_lost = false;
        Ok(())
    }
    fn install(
        &mut self,
        service: &Service,
        actor: &Actor,
        request: Request,
        snapshot: Snapshot,
    ) -> Result<()> {
        service.position(actor, request, &snapshot.cursor)?;
        let bytes =
            photara_core::canonical_json(&snapshot.roots).map_err(|_| ServiceError::Integrity)?;
        if snapshot.roots.len() > 10_000
            || bytes.len() > 16 * 1024 * 1024
            || hash(&bytes) != snapshot.canonical_sha256
        {
            return Err(ServiceError::Integrity);
        }
        let mut next = BTreeMap::new();
        for root in snapshot.roots {
            root.validate(request.scope)?;
            let (kind, id, _) = root.coordinates();
            if next.insert((kind, id), root).is_some() {
                return Err(ServiceError::Integrity);
            }
        }
        // Atomic swap after every root has validated; sealed working overlays remain separate.
        self.base = next;
        self.cursor = Some(snapshot.cursor);
        Ok(())
    }
    /// A dropped response models a server commit whose result has not reached the client.
    /// # Errors
    /// Conflicts and access loss retain sealed bytes; uncertain results remain reconcilable.
    pub async fn dispatch(
        &mut self,
        service: &Service,
        actor: &Actor,
        operation: OperationId,
        drop_response: bool,
    ) -> Result<()> {
        if actor.account() != self.actor || self.access_lost || self.cursor.is_none() {
            return Err(ServiceError::Forbidden);
        }
        let intent = self
            .intents
            .get_mut(&operation)
            .ok_or(ServiceError::Invalid)?;
        if canonical(&intent.request)? != intent.sealed {
            return Err(ServiceError::Integrity);
        }
        if matches!(
            intent.state,
            IntentState::Applied | IntentState::Acknowledged
        ) {
            return Ok(());
        }
        if intent.state == IntentState::Conflict {
            return Err(ServiceError::Conflict);
        }
        intent.state = IntentState::Unknown;
        match service
            .publish_content(actor, self.request, &intent.request)
            .await
        {
            Ok(receipt) => {
                if !drop_response {
                    intent.receipt = Some(receipt);
                    intent.state = IntentState::Acknowledged;
                }
                Ok(())
            }
            Err(ServiceError::Conflict) => {
                intent.state = IntentState::Conflict;
                Err(ServiceError::Conflict)
            }
            Err(ServiceError::Forbidden) => {
                self.access_lost = true;
                Err(ServiceError::Forbidden)
            }
            Err(e) => Err(e),
        }
    }
    /// # Errors
    /// Rejects a failed batch without cursor movement or partial root installation.
    pub async fn pull(&mut self, service: &Service, actor: &Actor) -> Result<()> {
        if actor.account() != self.actor || self.access_lost {
            return Err(ServiceError::Forbidden);
        }
        let before = self.cursor.as_ref().ok_or(ServiceError::Invalid)?;
        let page = match service.feed(actor, self.request, before, 100).await {
            Ok(v) => v,
            Err(ServiceError::Forbidden) => {
                self.access_lost = true;
                return Err(ServiceError::Forbidden);
            }
            Err(e) => return Err(e),
        };
        let mut next = self.base.clone();
        for batch in &page.batches {
            apply(&mut next, batch, self.request)?;
        }
        service.position(actor, self.request, &page.cursor)?;
        // Receipt reconciliation uses exact operation identity plus canonical root evidence.
        for batch in &page.batches {
            if let Some(receipt) = self
                .intents
                .get(&batch.operation)
                .and_then(|intent| intent.receipt.as_ref())
                && (receipt.operation != batch.operation
                    || batch.roots.len() != 1
                    || hash(&canonical(&batch.roots[0])?) != receipt.root_sha256)
            {
                return Err(ServiceError::Integrity);
            }
        }
        self.base = next;
        self.cursor = Some(page.cursor);
        for batch in page.batches {
            if let Some(intent) = self.intents.get_mut(&batch.operation)
                && intent.receipt.is_some()
            {
                intent.state = IntentState::Applied;
            }
        }
        Ok(())
    }
}
fn apply(
    base: &mut BTreeMap<(&'static str, Uuid), ContentRoot>,
    batch: &Batch,
    r: Request,
) -> Result<()> {
    if batch.scope != r.scope {
        return Err(ServiceError::Integrity);
    }
    for root in &batch.roots {
        root.validate(r.scope)?;
        let (kind, id, revision) = root.coordinates();
        let old = base.get(&(kind, id)).map(|v| v.coordinates().2);
        if old.map_or(revision != 1, |old| old.checked_add(1) != Some(revision)) {
            return Err(ServiceError::Conflict);
        }
        base.insert((kind, id), root.clone());
    }
    Ok(())
}
