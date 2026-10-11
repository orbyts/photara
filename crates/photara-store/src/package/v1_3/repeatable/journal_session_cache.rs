//! Optional deterministic semantic-prefix state. Physical journal proof remains
//! a prerequisite on every call; no owner, lease, Accepted or Saved is cached.
use super::super::super::restart::{self, CompiledPlans};
use super::{
    Arc, BTreeMap, BTreeSet, Journal, Limits, MutationEvidence, PackageError, PlannerInputs, Value,
    VerifiedClosure, VerifiedJournal, VerifiedOriginal, VerifiedSession, json, wire,
};

pub(crate) struct SessionMemo {
    key: String,
    prefix: Vec<u8>,
    count: usize,
    plans: BTreeMap<String, String>,
    state: Owned,
    pub(crate) cost: usize,
}
struct Owned {
    logical: Arc<VerifiedClosure>,
    head: Value,
    accepted: Value,
    journal_inclusion: Value,
    ordinal: u64,
    sequence: u64,
    mutations: BTreeMap<String, MutationEvidence>,
    historical: BTreeSet<String>,
    order: Vec<String>,
    checkpointed: usize,
    completed: Option<Value>,
    checkpoint_target: Value,
}
impl Owned {
    fn capture(v: &VerifiedSession<'_>) -> Self {
        Self {
            logical: v.logical.clone(),
            head: v.head.clone(),
            accepted: v.accepted.clone(),
            journal_inclusion: v.journal_inclusion.clone(),
            ordinal: v.ordinal,
            sequence: v.sequence,
            mutations: v.mutations.clone(),
            historical: v.historical.clone(),
            order: v.order.clone(),
            checkpointed: v.checkpointed,
            completed: v.completed.clone(),
            checkpoint_target: v.checkpoint_target.clone(),
        }
    }
    fn attach<'a>(&self, journal: &'a Journal) -> VerifiedSession<'a> {
        VerifiedSession {
            journal,
            logical: self.logical.clone(),
            head: self.head.clone(),
            accepted: self.accepted.clone(),
            journal_inclusion: self.journal_inclusion.clone(),
            ordinal: self.ordinal,
            sequence: self.sequence,
            mutations: self.mutations.clone(),
            historical: self.historical.clone(),
            order: self.order.clone(),
            checkpointed: self.checkpointed,
            completed: self.completed.clone(),
            checkpoint_target: self.checkpoint_target.clone(),
        }
    }
    fn cost(&self) -> Option<usize> {
        let metadata = json!([
            self.head,
            self.accepted,
            self.journal_inclusion,
            self.historical,
            self.order,
            self.completed,
            self.checkpoint_target,
            self.mutations
                .iter()
                .map(|(id, m)| json!([
                    id,
                    m.record.value,
                    m.before.iter().collect::<Vec<_>>(),
                    m.after.iter().collect::<Vec<_>>(),
                    m.accepted,
                    m.journal_inclusion,
                    m.undone_by,
                    m.invalidated
                ]))
                .collect::<Vec<_>>()
        ]);
        self.logical
            .memo_bytes(restart::value_storage)?
            .checked_add(restart::value_storage(&metadata)?)?
            .checked_add(1024)
    }
}
fn basis(base: &VerifiedOriginal, inputs: &PlannerInputs, limits: Limits) -> String {
    let p = base.shared_package();
    wire::hash(&wire::encode(
        &json!({"manifest":p.manifest,"head":p.head,"commit":p.commit,
        "loose":p.loose.iter().map(|(k,b)|json!([k,wire::hash(b),b.len()])).collect::<Vec<_>>(),
        "allocations":p.allocations.iter().map(|(k,a)|json!([k,a.arena,a.witness,a.bytes.len(),wire::hash(&a.bytes)])).collect::<Vec<_>>(),
        "roles":base.proof.roles.iter().map(|(k,r)|json!([k,r.state,r.semantic,r.receipts])).collect::<Vec<_>>(),
        "global":base.proof.global,"scope":base.scope,"charge":base.charge_unit,"empty":base.empty_hold,
        "retained":base.retained_controls.iter().map(|(k,m)|json!([k,format!("{m:?}")])).collect::<Vec<_>>(),
        "inputs":inputs,"limits":restart::planner_limits(limits)}),
    ))
}
fn plan_key(p: &super::super::CheckpointPlan) -> String {
    wire::hash(&wire::encode(&json!([
        p.original,
        p.intent,
        p.receipt,
        p.selected_state,
        p.head,
        format!("{:?}", p.json_shape)
    ])))
}
impl<'a> VerifiedJournal<'a> {
    pub(crate) fn verify_session_cached(
        &self,
        base: &VerifiedOriginal,
        inputs: &PlannerInputs,
        limits: Limits,
        memo: &mut CompiledPlans,
    ) -> Result<VerifiedSession<'a>, PackageError> {
        memo.trim(limits, usize::MAX);
        let key = basis(base, inputs, limits);
        let prior = memo.session.as_ref().filter(|entry| {
            entry.key == key
                && self.journal.bytes.starts_with(&entry.prefix)
                && entry
                    .plans
                    .iter()
                    .all(|(id, key)| self.plans.get(id).is_some_and(|p| plan_key(p) == *key))
        });
        let start = prior.map_or(0, |entry| entry.count);
        let initial = match prior {
            Some(entry) => entry.state.attach(self.journal),
            None => self.initial_session(base, inputs, limits)?,
        };
        let session = self.extend_session(initial, start, limits)?;
        #[cfg(test)]
        {
            memo.session_replayed = self.journal.records.len() - start;
            memo.session_record_work += memo.session_replayed;
        }
        let state = Owned::capture(&session);
        let prefix = self.journal.bytes[..self.journal.verified_bytes].to_vec();
        let plans = self
            .plans
            .iter()
            .map(|(id, p)| (id.clone(), plan_key(p)))
            .collect::<BTreeMap<_, _>>();
        let cost = state
            .cost()
            .and_then(|n| n.checked_add(prefix.capacity()))
            .and_then(|n| n.checked_add(key.capacity()))
            .and_then(|n| n.checked_add(restart::value_storage(&json!(plans))?));
        if let Some(cost) = cost.filter(|n| *n <= memo.raw_maximum(limits) / 4) {
            memo.session = Some(SessionMemo {
                key,
                prefix,
                count: self.journal.records.len(),
                plans,
                state,
                cost,
            });
        }
        memo.trim(limits, usize::MAX);
        Ok(session)
    }
}
