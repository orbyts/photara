//! Transport-neutral logical-client model, not a production coordinator, journal,
//! authorization service, OS lock or durability receipt. All state is in memory;
//! only semantic planning/candidate verification uses the real PS1 implementation.
use super::*;
use photara_core::{GraphId, contracts::ids::OperationId};

#[derive(Clone)]
struct HostGrant {
    principal: u8,
    grantor: u8,
    incarnation: IncarnationId,
    graph: GraphId,
    generation: u64,
    active: bool,
}
#[derive(Clone, Copy)]
struct Attachment {
    id: u8,
    owner: OwnerEpoch,
}
#[derive(Clone)]
struct AttachedGrant {
    grant: u8,
    generation: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct ModelReceipt {
    sequence: u64,
    intent: String,
    principal: u8,
    grantor: u8,
    grant: u8,
    grant_generation: u64,
    graph: GraphId,
    after: AuthoredCoordinate,
}
struct Accepted {
    operation: OperationId,
    receipt: ModelReceipt,
    candidate: BTreeMap<String, Vec<u8>>,
}
#[derive(Debug, Eq, PartialEq)]
enum Refusal {
    WriterBusy,
    Attachment,
    Unauthorized,
    Scope,
    IntentConflict,
    Stale,
    Cursor,
}
#[derive(Debug, Eq, PartialEq)]
enum Feed {
    Events(Vec<ModelReceipt>),
    Gap {
        cursor: u64,
        snapshot: AuthoredCoordinate,
    },
}
struct Model {
    owner: Option<OwnerEpoch>,
    grants: BTreeMap<u8, HostGrant>,
    attachments: BTreeMap<u8, AttachedGrant>,
    next_attachment: u8,
    current: VerifiedClosure,
    accepted: Vec<Accepted>,
    checkpointed: u64,
}
impl Model {
    fn new() -> Self {
        let grant = HostGrant {
            principal: 1,
            grantor: 9,
            incarnation: incarnation(),
            graph: decode(json!(id(20018))),
            generation: 1,
            active: true,
        };
        Self {
            owner: None,
            grants: BTreeMap::from([
                (10, grant.clone()),
                (
                    20,
                    HostGrant {
                        principal: 2,
                        ..grant
                    },
                ),
            ]),
            attachments: BTreeMap::new(),
            next_attachment: 0,
            current: verified(build(add_history)),
            accepted: Vec::new(),
            checkpointed: 0,
        }
    }
    fn claim_owner(&mut self, owner: OwnerEpoch) -> Result<(), Refusal> {
        if self.owner.is_some() {
            return Err(Refusal::WriterBusy);
        }
        self.owner = Some(owner);
        Ok(())
    }
    // `host_principal` is a trusted synthetic host input, NOT a wire-auth scheme.
    fn attach(&mut self, host_principal: u8, grant_id: u8) -> Result<Attachment, Refusal> {
        let grant = self.grants.get(&grant_id).ok_or(Refusal::Unauthorized)?;
        if grant.principal != host_principal || !grant.active {
            return Err(Refusal::Unauthorized);
        }
        let owner = self.owner.ok_or(Refusal::Attachment)?;
        self.next_attachment += 1;
        self.attachments.insert(
            self.next_attachment,
            AttachedGrant {
                grant: grant_id,
                generation: grant.generation,
            },
        );
        Ok(Attachment {
            id: self.next_attachment,
            owner,
        })
    }
    fn authorize(&self, attachment: Attachment) -> Result<(u8, &HostGrant), Refusal> {
        if Some(attachment.owner) != self.owner {
            return Err(Refusal::Attachment);
        }
        let attached = self
            .attachments
            .get(&attachment.id)
            .ok_or(Refusal::Attachment)?;
        let grant = self
            .grants
            .get(&attached.grant)
            .ok_or(Refusal::Unauthorized)?;
        if !grant.active || grant.generation != attached.generation {
            return Err(Refusal::Unauthorized);
        }
        if grant.incarnation != incarnation() || grant.graph != decode(json!(id(20018))) {
            return Err(Refusal::Scope);
        }
        Ok((attached.grant, grant))
    }
    fn submit(
        &mut self,
        attachment: Attachment,
        request: &MutationRequest,
    ) -> Result<ModelReceipt, Refusal> {
        let (grant_id, grant) = self.authorize(attachment)?;
        let AuthoredCommand::RenameGraph { graph_id, .. } = &request.command else {
            return Err(Refusal::Scope);
        };
        if graph_id != &grant.graph {
            return Err(Refusal::Scope);
        }
        let intent = hash(&canonical_json(request).unwrap());
        if let Some(old) = self
            .accepted
            .iter()
            .find(|item| item.operation == request.operation_id)
        {
            if old.receipt.principal != grant.principal {
                return Err(Refusal::Unauthorized);
            }
            if old.receipt.intent != intent {
                return Err(Refusal::IntentConflict);
            }
            return Ok(old.receipt.clone()); // Before stale-state rejection, after authorization.
        }
        if &request.expected != self.current.coordinate() {
            return Err(Refusal::Stale);
        }
        let sequence = self.accepted.len() as u64 + 1;
        let plan = checkpoint(
            run(
                &self.current,
                request,
                40000 + u32::try_from(sequence).unwrap() * 2,
            )
            .unwrap(),
        );
        let receipt = ModelReceipt {
            sequence,
            intent,
            principal: grant.principal,
            grantor: grant.grantor,
            grant: grant_id,
            grant_generation: grant.generation,
            graph: grant.graph,
            after: plan.receipt().after().clone(),
        };
        assert_eq!(receipt.intent, plan.receipt().request_digest().as_str());
        let candidate = owned(plan.candidate().files());
        self.current = verified(candidate.clone());
        self.accepted.push(Accepted {
            operation: request.operation_id,
            receipt: receipt.clone(),
            candidate,
        });
        Ok(receipt)
    }
    fn events(&self, client: Attachment, after: u64) -> Result<Feed, Refusal> {
        self.authorize(client)?;
        let current = self.accepted.len() as u64;
        if after > current {
            return Err(Refusal::Cursor);
        }
        // Window of two is fixture-only, not a proposed production retention limit.
        if after < current.saturating_sub(2) {
            return Ok(Feed::Gap {
                cursor: current,
                snapshot: self.current.coordinate().clone(),
            });
        }
        Ok(Feed::Events(
            self.accepted
                .iter()
                .filter(|item| item.receipt.sequence > after)
                .map(|item| item.receipt.clone())
                .collect(),
        ))
    }
    // Select and verify a finite VIRTUAL checkpoint prefix; no filesystem publish.
    fn checkpoint_prefix(
        &mut self,
        client: Attachment,
        target: u64,
    ) -> Result<ModelReceipt, Refusal> {
        self.authorize(client)?;
        let index = usize::try_from(target)
            .ok()
            .and_then(|n| n.checked_sub(1))
            .ok_or(Refusal::Cursor)?;
        let accepted = self.accepted.get(index).ok_or(Refusal::Cursor)?;
        assert_eq!(
            verified(accepted.candidate.clone()).coordinate(),
            &accepted.receipt.after
        );
        self.checkpointed = self.checkpointed.max(target);
        Ok(accepted.receipt.clone())
    }
    fn detach(&mut self, client: Attachment) -> Result<(), Refusal> {
        self.authorize(client)?;
        self.attachments.remove(&client.id);
        Ok(())
    }
}

fn owner(n: u32) -> OwnerEpoch {
    OwnerEpoch::parse(&id(n)).unwrap()
}
fn command(model: &Model, n: u32, name: &str) -> MutationRequest {
    let mut request = rename(&model.current, name);
    request.operation_id = decode(json!(id(n)));
    request
}

#[test]
fn two_clients_order_dedupe_reconnect_flush_and_detach() {
    let mut model = Model::new();
    model.claim_owner(owner(39000)).unwrap();
    let gui = model.attach(1, 10).unwrap();
    let agent = model.attach(2, 20).unwrap();
    assert_eq!(model.claim_owner(owner(39001)), Err(Refusal::WriterBusy));
    let first = command(&model, 41000, "GUI edit");
    let stale_agent = command(&model, 41001, "Agent edit");
    let receipt1 = model.submit(gui, &first).unwrap();
    assert_eq!(model.submit(agent, &stale_agent), Err(Refusal::Stale));
    assert_eq!(model.submit(gui, &first), Ok(receipt1.clone()));
    let mut changed = first.clone();
    changed.command = rename(&model.current, "Other intent").command;
    assert_eq!(model.submit(gui, &changed), Err(Refusal::IntentConflict));
    assert_eq!(model.submit(agent, &first), Err(Refusal::Unauthorized));
    let second = command(&model, 41001, "Agent edit");
    let receipt2 = model.submit(agent, &second).unwrap();
    assert_eq!(
        model.events(gui, 0),
        Ok(Feed::Events(vec![receipt1.clone(), receipt2.clone()]))
    );
    let third = command(&model, 41002, "Later agent edit");
    let receipt3 = model.submit(agent, &third).unwrap();
    assert_eq!(model.checkpoint_prefix(gui, 1), Ok(receipt1.clone()));
    assert_eq!(model.checkpointed, 1);
    assert_ne!(model.checkpointed, model.accepted.len() as u64); // Newer state is not Saved.
    model.detach(gui).unwrap();
    assert_eq!(model.events(gui, 0), Err(Refusal::Attachment));
    assert_eq!(model.submit(agent, &second), Ok(receipt2.clone()));
    let reconnected = model.attach(1, 10).unwrap();
    assert_eq!(model.submit(reconnected, &first), Ok(receipt1));
    assert_eq!(
        model.events(reconnected, 0),
        Ok(Feed::Gap {
            cursor: 3,
            snapshot: receipt3.after.clone()
        })
    );
    assert_eq!(
        model.events(reconnected, 1),
        Ok(Feed::Events(vec![receipt2, receipt3.clone()]))
    );
    assert_eq!(model.events(reconnected, 4), Err(Refusal::Cursor));
    assert_eq!(model.checkpoint_prefix(agent, 3), Ok(receipt3));
    assert_eq!(model.accepted.len(), 3);
    assert_eq!(model.current.coordinate().revision.get(), 4);
}

#[test]
fn host_scope_revocation_and_stale_attachment_refuse_without_effects() {
    let mut model = Model::new();
    model.claim_owner(owner(39000)).unwrap();
    assert!(matches!(model.attach(2, 10), Err(Refusal::Unauthorized)));
    assert!(matches!(model.attach(2, 99), Err(Refusal::Unauthorized)));
    let agent = model.attach(2, 20).unwrap();
    let first = command(&model, 41000, "Agent edit");
    let accepted = model.submit(agent, &first).unwrap();
    assert_eq!(
        (accepted.principal, accepted.grantor, accepted.grant),
        (2, 9, 20)
    );
    let before = owned(model.current.files());
    let next = command(&model, 41001, "Refused edit");
    let stale = Attachment {
        owner: owner(39001),
        ..agent
    };
    assert_eq!(model.submit(stale, &next), Err(Refusal::Attachment));
    model.grants.get_mut(&20).unwrap().incarnation = IncarnationId::parse(&id(39002)).unwrap();
    assert_eq!(model.submit(agent, &next), Err(Refusal::Scope));
    model.grants.get_mut(&20).unwrap().incarnation = incarnation();
    model.grants.get_mut(&20).unwrap().graph = decode(json!(id(39003)));
    assert_eq!(model.submit(agent, &next), Err(Refusal::Scope));
    model.grants.get_mut(&20).unwrap().graph = decode(json!(id(20018)));
    model.grants.get_mut(&20).unwrap().generation += 1;
    assert_eq!(model.submit(agent, &next), Err(Refusal::Unauthorized));
    let renewed = model.attach(2, 20).unwrap();
    assert_eq!(model.submit(renewed, &first), Ok(accepted.clone()));
    model.grants.get_mut(&20).unwrap().active = false;
    assert_eq!(model.submit(renewed, &first), Err(Refusal::Unauthorized));
    assert_eq!(model.events(renewed, 0), Err(Refusal::Unauthorized));
    assert_eq!(model.accepted[0].receipt, accepted); // Historical acceptance survives revocation.
    assert_eq!(model.accepted.len(), 1);
    assert_eq!(owned(model.current.files()), before);
}
