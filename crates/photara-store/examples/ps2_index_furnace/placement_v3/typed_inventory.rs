//! Disposable typed semantic/ownership closure in the actual v3 locator.
#[allow(
    clippy::wildcard_imports,
    reason = "Fixture child shares actual v3 internals"
)]
use super::*;
use std::os::unix::fs::MetadataExt;
#[path = "typed_inventory/recipe.rs"]
pub(super) mod recipe;
pub(super) const OWNER_KEY: u64 = 1 << 63;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) enum Membership {
    Semantic,
    Ownership,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Inventory {
    pub active: Option<Ref>,
    pub recovery: Option<Ref>,
    pub next: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Claim {
    data: bool,
    pack: u64,
    dev: u64,
    ino: u64,
    extent: u64,
    sha: String,
    sealed: bool,
}
impl Claim {
    fn key(&self) -> Result<u64> {
        self.pack
            .checked_mul(2)
            .and_then(|v| v.checked_add(u64::from(self.data)))
            .ok_or("allocation key overflow".into())
    }
    fn capture(f: &Fixture, data: bool, pack: u64) -> Result<Self> {
        let a = if data { &f.data } else { &f.meta };
        let m = fs::symlink_metadata(a.path(pack)).map_err(error)?;
        ensure(
            m.is_file() && m.nlink() == 1 && m.len() <= PACK,
            "private bounded allocation",
        )?;
        Ok(Self {
            data,
            pack,
            dev: m.dev(),
            ino: m.ino(),
            extent: m.len(),
            sha: hash(&fs::read(a.path(pack)).map_err(error)?),
            sealed: pack < a.pack,
        })
    }
    fn verify(&self, f: &Fixture) -> Result<()> {
        ensure(
            self.extent <= PACK && self.sha.len() == 64,
            "ownership claim shape",
        )?;
        let a = if self.data { &f.data } else { &f.meta };
        let m = fs::symlink_metadata(a.path(self.pack)).map_err(error)?;
        ensure(
            m.is_file()
                && m.nlink() == 1
                && m.dev() == self.dev
                && m.ino() == self.ino
                && m.len() >= self.extent
                && m.len() <= PACK
                && (!self.sealed || m.len() == self.extent),
            "ownership generation/prefix mismatch",
        )?;
        let mut bytes = vec![0; usize::try_from(self.extent).map_err(error)?];
        File::open(a.path(self.pack))
            .and_then(|mut f| f.read_exact(&mut bytes))
            .map_err(error)?;
        ensure(hash(&bytes) == self.sha, "owned prefix corruption")
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
enum OwnedNode {
    Leaf {
        claim: Claim,
    },
    Branch {
        bit: u8,
        anchor: u64,
        left: Ref,
        right: Ref,
    },
}
impl OwnedNode {
    fn anchor(&self) -> Result<u64> {
        match self {
            Self::Leaf { claim } => claim.key(),
            Self::Branch { anchor, .. } => Ok(*anchor),
        }
    }
}
impl Fixture {
    fn ownership_root(&self, root: &PRef) -> Result<Option<Ref>> {
        let mut matches = vec![];
        let mut consider = |a: &PRef, b: &PRef, inv: &Inventory| {
            if a == root {
                matches.push(inv.active.clone());
            }
            if b == root {
                matches.push(inv.recovery.clone());
            }
        };
        consider(&self.head.active, &self.head.recovery, &self.head.inventory);
        for pin in self.head.gate.pins.values() {
            consider(&pin.active, &pin.recovery, &pin.inventory);
        }
        for hold in self.head.gate.holds.values() {
            consider(
                &hold.origin.active,
                &hold.origin.recovery,
                &hold.origin.inventory,
            );
            if let Some(c) = &hold.continuation {
                consider(&c.active, &c.recovery, &c.inventory);
            }
        }
        let result = matches.first().cloned().unwrap_or(None);
        ensure(
            matches.iter().all(|v| *v == result),
            "ambiguous selected inventory commitment",
        )?;
        Ok(result)
    }
    pub(super) fn selected_inventory(&self, active: &PRef, recovery: &PRef) -> Result<Inventory> {
        let mut next = self.head.inventory.next;
        for hold in self.head.gate.holds.values() {
            if let Some(c) = &hold.continuation {
                next = next.max(c.inventory.next);
            }
        }
        let known = |r: &PRef| {
            self.head.active == *r
                || self.head.recovery == *r
                || self.head.gate.holds.values().any(|h| {
                    h.continuation
                        .as_ref()
                        .is_some_and(|c| c.active == *r || c.recovery == *r)
                })
        };
        Ok(Inventory {
            active: if known(active) {
                self.ownership_root(active)?
            } else {
                self.head.inventory.active.clone()
            },
            recovery: if known(recovery) {
                self.ownership_root(recovery)?
            } else {
                self.head.inventory.recovery.clone()
            },
            next,
        })
    }
    fn owned_node(&mut self, root: &PRef, r: &Ref) -> Result<OwnedNode> {
        let next = self
            .head
            .gate
            .holds
            .values()
            .filter_map(|h| h.continuation.as_ref().map(|c| c.inventory.next))
            .fold(self.head.inventory.next, u64::max);
        ensure(
            r.offset >= OWNER_KEY && r.offset - OWNER_KEY < next && r.len <= 4092,
            "ownership reference namespace/size",
        )?;
        let loc = self
            .lookup(root, r.offset)?
            .ok_or("missing ownership object")?;
        ensure(
            loc.membership == Membership::Ownership && loc.object == *r,
            "ownership membership/identity",
        )?;
        ensure(
            loc.data.pack < self.head.data_pack
                || (loc.data.pack == self.head.data_pack
                    && loc
                        .data
                        .offset
                        .checked_add(loc.data.len)
                        .is_some_and(|e| e <= self.head.data_end)),
            "ownership outside selected prefix",
        )?;
        let bytes = self.data.read(&loc.data)?;
        self.c.data_reads += 1;
        self.c.data_read_bytes += bytes.len() as u64;
        ensure(hash(&bytes) == r.sha, "ownership content commitment")?;
        serde_json::from_slice(&bytes).map_err(error)
    }
    fn ownership_lookup(&mut self, root: &PRef, key: u64) -> Result<Option<Claim>> {
        let Some(mut at) = self.ownership_root(root)? else {
            return Ok(None);
        };
        let mut parent = None;
        for _ in 0..=64 {
            match self.owned_node(root, &at)? {
                OwnedNode::Leaf { claim } => return Ok((claim.key()? == key).then_some(claim)),
                OwnedNode::Branch {
                    bit,
                    anchor,
                    left,
                    right,
                } => {
                    ensure(
                        bit < 64 && parent.is_none_or(|p| p < bit),
                        "ownership branch order",
                    )?;
                    if difference(key, anchor) < bit {
                        return Ok(None);
                    }
                    parent = Some(bit);
                    at = if direction(key, bit) { right } else { left };
                }
            }
        }
        Err("ownership path bound".into())
    }
    pub(super) fn audit_inventory(&mut self, root: &PRef) -> Result<HashSet<Ref>> {
        let mut seen = HashSet::new();
        let mut keys = HashSet::new();
        let mut todo = self
            .ownership_root(root)?
            .map(|r| vec![(r, Vec::<(u8, u64, bool)>::new())])
            .unwrap_or_default();
        while let Some((r, path)) = todo.pop() {
            ensure(seen.insert(r.clone()), "duplicate/cyclic ownership node")?;
            ensure(path.len() <= 64, "ownership audit depth")?;
            let node = self.owned_node(root, &r)?;
            let anchor = node.anchor()?;
            ensure(
                path.iter().all(|(bit, a, right)| {
                    difference(anchor, *a) >= *bit && direction(anchor, *bit) == *right
                }),
                "ownership routing mismatch",
            )?;
            match node {
                OwnedNode::Leaf { claim } => {
                    ensure(keys.insert(claim.key()?), "duplicate allocation claim")?;
                    claim.verify(self)?;
                }
                OwnedNode::Branch {
                    bit,
                    anchor,
                    left,
                    right,
                } => {
                    ensure(
                        bit < 64 && path.last().is_none_or(|(b, _, _)| *b < bit),
                        "malformed ownership branch",
                    )?;
                    let mut lp = path.clone();
                    lp.push((bit, anchor, false));
                    let mut rp = path;
                    rp.push((bit, anchor, true));
                    todo.push((left, lp));
                    todo.push((right, rp));
                }
            }
        }
        Ok(seen)
    }
    pub(super) fn inventory_probe(&mut self, root: &PRef) -> Result<()> {
        if let Some(r) = self.ownership_root(root)? {
            let n = self.owned_node(root, &r)?;
            if let OwnedNode::Branch { bit, .. } = n {
                ensure(bit < 64, "ownership root shape")?;
            }
        }
        Ok(())
    }
    pub(super) fn inventory_retirement_guard(&mut self, pack: u64, data: bool) -> Result<()> {
        let key = pack
            .checked_mul(2)
            .and_then(|v| v.checked_add(u64::from(data)))
            .ok_or("candidate identity overflow")?;
        let mut roots = vec![self.head.active.clone(), self.head.recovery.clone()];
        for p in self.head.gate.pins.values() {
            roots.extend([p.active.clone(), p.recovery.clone()]);
        }
        for r in roots {
            ensure(
                self.ownership_lookup(&r, key)?.is_none(),
                "exact selected/pinned ownership claim blocks retirement",
            )?;
        }
        Ok(())
    }
}

struct Builder {
    data: WritePlan,
    next: u64,
    added: Vec<(Ref, Location)>,
    removed: Vec<Ref>,
}
impl Builder {
    fn put(&mut self, node: &OwnedNode) -> Result<Ref> {
        let bytes = serde_json::to_vec(node).map_err(error)?;
        ensure(bytes.len() <= 4092, "ownership node size")?;
        let offset = OWNER_KEY
            .checked_add(self.next)
            .ok_or("ownership ID exhaustion")?;
        self.next = checked(self.next, 1)?;
        let r = Ref {
            offset,
            len: bytes.len() as u64,
            sha: hash(&bytes),
        };
        let data = self.data.push(bytes, Some(offset))?;
        self.added.push((
            r.clone(),
            Location {
                membership: Membership::Ownership,
                object: r.clone(),
                data,
            },
        ));
        Ok(r)
    }
    fn insert(
        &mut self,
        f: &mut Fixture,
        locator: &PRef,
        at: Option<Ref>,
        claim: &Claim,
    ) -> Result<Ref> {
        ensure(self.removed.len() < 65, "bounded ownership insertion path")?;
        let Some(at) = at else {
            return self.put(&OwnedNode::Leaf {
                claim: claim.clone(),
            });
        };
        let node = f.owned_node(locator, &at)?;
        let key = claim.key()?;
        let anchor = node.anchor()?;
        let branch = match node {
            OwnedNode::Branch { bit, .. } => bit,
            OwnedNode::Leaf { .. } => 64,
        };
        let diff = difference(anchor, key);
        if diff < branch {
            let leaf = self.put(&OwnedNode::Leaf {
                claim: claim.clone(),
            })?;
            let (left, right) = if direction(key, diff) {
                (at, leaf)
            } else {
                (leaf, at)
            };
            return self.put(&OwnedNode::Branch {
                bit: diff,
                anchor: anchor.min(key),
                left,
                right,
            });
        }
        self.removed.push(at);
        match node {
            OwnedNode::Leaf { claim: old } => {
                ensure(
                    old.key()? == key
                        && old.dev == claim.dev
                        && old.ino == claim.ino
                        && claim.extent >= old.extent,
                    "conflicting ownership generation",
                )?;
                self.put(&OwnedNode::Leaf {
                    claim: claim.clone(),
                })
            }
            OwnedNode::Branch {
                bit,
                anchor,
                left,
                right,
            } => {
                let (left, right) = if direction(key, bit) {
                    (left, self.insert(f, locator, Some(right), claim)?)
                } else {
                    (self.insert(f, locator, Some(left), claim)?, right)
                };
                self.put(&OwnedNode::Branch {
                    bit,
                    anchor,
                    left,
                    right,
                })
            }
        }
    }
    fn updates(&self) -> Vec<(u64, Option<Location>)> {
        let mut updates: BTreeMap<_, _> = self.removed.iter().map(|r| (r.offset, None)).collect();
        updates.extend(self.added.iter().map(|(r, l)| (r.offset, Some(l.clone()))));
        updates.into_iter().collect()
    }
}
struct TypedPlan {
    data: WritePlan,
    meta: WritePlan,
    continuation: Continuation,
}
fn plan(f: &mut Fixture, claim: &Claim) -> Result<TypedPlan> {
    ensure(
        !f.imported_read_only,
        "imported inventory requires explicit full audit",
    )?;
    claim.verify(f)?;
    ensure(f.head.gate.holds.is_empty(), "pending original operation")?;
    let h = f.head.clone();
    let mut a = Builder {
        data: WritePlan::from(&f.data),
        next: h.inventory.next,
        added: vec![],
        removed: vec![],
    };
    let ar = a.insert(f, &h.active, h.inventory.active.clone(), claim)?;
    let au = a.updates();
    let (data, next, br, bu) = if h.inventory.active == h.inventory.recovery {
        (a.data, a.next, ar.clone(), au.clone())
    } else {
        let mut b = Builder {
            data: a.data,
            next: a.next,
            added: vec![],
            removed: vec![],
        };
        let br = b.insert(f, &h.recovery, h.inventory.recovery.clone(), claim)?;
        let bu = b.updates();
        (b.data, b.next, br, bu)
    };
    f.staged = Some(Vec::new());
    let active = f
        .multi(Some(h.active.clone()), &au)?
        .ok_or("active typed locator")?;
    let recovery = f
        .multi(Some(h.recovery.clone()), &bu)?
        .ok_or("recovery typed locator")?;
    let pages = f.staged.take().ok_or("typed staged pages")?;
    let mut meta = WritePlan::from(&f.meta);
    let mut memo = HashMap::new();
    let active = meta.staged(&active, &pages, &mut memo)?;
    let recovery = meta.staged(&recovery, &pages, &mut memo)?;
    let continuation = Continuation {
        recipe: None,
        inventory: Inventory {
            active: Some(ar),
            recovery: Some(br),
            next,
        },
        active,
        recovery,
        data_pack: data.pack,
        data_end: data.end,
        meta_pack: meta.pack,
        meta_end: meta.end,
    };
    Ok(TypedPlan {
        data,
        meta,
        continuation,
    })
}
fn publish_claim(f: &mut Fixture, claim: &Claim, cut: Cut) -> Result<serde_json::Value> {
    let p = plan(f, claim)?;
    let bound = physical_bound(Some(&p.data), &p.meta)?;
    let budget = checked(
        bound["allocation_bound"].as_u64().ok_or("typed reserve")?,
        4 * f.control_bound(&f.head.gate)?,
    )?;
    let hold = f.reserve_with(
        f.head.semantic.clone(),
        None,
        &[(0, budget)],
        Some(p.continuation.clone()),
    )?;
    f.authorized_hold = Some(hold.token);
    let before = f.c.clone();
    p.data.write(&mut f.data, &mut f.c)?;
    p.meta.write(&mut f.meta, &mut f.c)?;
    f.publish(
        p.continuation.active,
        p.continuation.recovery,
        hold.target.clone(),
        cut,
    )?;
    let used = f.c.allocation_charge - before.allocation_charge
        + (f.c.files_created - before.files_created) * 16384;
    f.complete_liability(&hold, &[(0, used)])?;
    Ok(json!({"preflight":bound,"original_hold":hold.by_domain,"used_model":used}))
}
pub(super) fn run_cli(args: &[String]) -> Result<()> {
    if args.first().is_some_and(|s| s == "recipe") {
        return recipe::run_cli(&args[1..]);
    }
    for n in if args.is_empty() {
        vec![1000]
    } else {
        args.iter()
            .map(|n| n.parse().map_err(error))
            .collect::<Result<Vec<u64>>>()?
    } {
        ensure((1..=10000).contains(&n), "bounded typed fixture size")?;
        for kind in [Kind::Radix, Kind::Btree] {
            let start = Instant::now();
            let (_temp, mut source, mut f) = setup(n, kind)?;
            let semantic = f.head.semantic.clone();
            let mut installs = vec![];
            for (data, pack) in [(true, 0), (false, 0)] {
                let claim = Claim::capture(&f, data, pack)?;
                let before = f.snapshot()?;
                f.c = Counters::default();
                let t = Instant::now();
                let result = publish_claim(&mut f, &claim, Cut::None)?;
                ensure(
                    f.head.semantic == semantic,
                    "ownership publication changed authored identity",
                )?;
                installs.push(json!({"data":data,"us":t.elapsed().as_micros(),"counters":f.c,"before":before,"after":f.snapshot()?,"result":result}));
            }
            let mut samples = vec![];
            for group in [1, 8, 32] {
                let end = source.size();
                logical_group(&mut source, group)?;
                f.c = Counters::default();
                let before = f.snapshot()?;
                let t = Instant::now();
                let result = f.checkpoint(&mut source, end, Fault::None)?;
                samples.push(json!({"group":group,"us":t.elapsed().as_micros(),"counters":f.c,"before":before,"after":f.snapshot()?,"result":result}));
            }
            let t = Instant::now();
            let mut opened = Fixture::reopen_combined(&f.dir)?;
            opened.structural_probe()?;
            let open = json!({"us":t.elapsed().as_micros(),"counters":opened.c});
            opened.c = Counters::default();
            let t = Instant::now();
            opened.retry(&id(1), &request(&id(1)))?;
            let retry = json!({"us":t.elapsed().as_micros(),"counters":opened.c});
            let t = Instant::now();
            let count = f.audit_combined()?;
            let audit_us = t.elapsed().as_micros();
            println!(
                "{}",
                json!({"n":n,"kind":format!("{kind:?}"),"fully_materialized":true,"selected_ownership_claims":2,"installs":installs,"samples":samples,"structural_open":open,"old_retry":retry,"audit_us":audit_us,"audited_objects":count,"elapsed_ms":start.elapsed().as_millis(),"typed_closure_integrated":true,"complete_all_pack_charge_refund":false,"qualified_saved":false,"external_media_reads":0})
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_claim_survives_audit_checkpoint_and_structural_reopen() -> Result<()> {
        let (_temp, mut source, mut f) = setup(128, Kind::Radix)?;
        let semantic = f.head.semantic.clone();
        let claim = Claim::capture(&f, true, 0)?;
        publish_claim(&mut f, &claim, Cut::None)?;
        ensure(
            f.head.semantic == semantic,
            "physical inventory changed semantic identity",
        )?;
        f.audit_combined()?;
        let inv = f.head.inventory.active.clone();
        let end = source.size();
        logical_group(&mut source, 8)?;
        f.checkpoint(&mut source, end, Fault::None)?;
        ensure(
            f.head.inventory.active == inv && f.head.inventory.recovery == inv,
            "checkpoint dropped ownership root",
        )?;
        f = Fixture::reopen_combined(&f.dir)?;
        f.structural_probe()?;
        f.audit_combined()?;
        ensure(
            f.ownership_lookup(&f.head.active.clone(), claim.key()?)? == Some(claim),
            "typed claim absent",
        )
    }
    #[test]
    fn original_typed_hold_survives_before_and_after_head_cuts() -> Result<()> {
        for cut in [Cut::BeforeHead, Cut::AfterHead] {
            let (_temp, mut source, mut f) = setup(128, Kind::Btree)?;
            let semantic = f.head.semantic.clone();
            let claim = Claim::capture(&f, true, 0)?;
            ensure(publish_claim(&mut f, &claim, cut).is_err(), "missing fault")?;
            let hold = f.head.gate.holds.values().next().unwrap().clone();
            f = Fixture::reopen_combined(&f.dir)?;
            ensure(
                f.head.gate.holds.get(&hold.token) == Some(&hold),
                "original hold absent",
            )?;
            f.resume_checkpoint(&mut source, 0, &hold)?;
            f.audit_combined()?;
            ensure(
                f.head.semantic == semantic && f.head.gate.holds.is_empty(),
                "typed recovery changed semantics or lost completion",
            )?;
        }
        Ok(())
    }
    #[test]
    fn forged_missing_and_duplicate_typed_locator_leaves_refuse() -> Result<()> {
        for attack in 0..3 {
            let (_temp, _source, mut f) = setup(64, Kind::Radix)?;
            let claim = Claim::capture(&f, true, 0)?;
            publish_claim(&mut f, &claim, Cut::None)?;
            let root = f.head.active.clone();
            let r = f.head.inventory.active.clone().unwrap();
            let mut loc = f.lookup(&root, r.offset)?.unwrap();
            let (key, value) = match attack {
                0 => {
                    loc.membership = Membership::Semantic;
                    (r.offset, Some(loc))
                }
                1 => (r.offset, None),
                _ => {
                    loc.object.offset += 100;
                    (loc.object.offset, Some(loc))
                }
            };
            let changed = f.update(Some(root), key, value)?.ok_or("missing locator")?;
            f.publish(
                changed,
                f.head.recovery.clone(),
                f.head.semantic.clone(),
                Cut::None,
            )?;
            ensure(
                f.audit_combined().is_err(),
                "typed inventory forgery passed",
            )?;
        }
        Ok(())
    }
    #[test]
    fn corrupt_ownership_payload_blocks_original_continuation() -> Result<()> {
        let (_temp, mut source, mut f) = setup(128, Kind::Radix)?;
        let claim = Claim::capture(&f, true, 0)?;
        ensure(
            publish_claim(&mut f, &claim, Cut::BeforeHead).is_err(),
            "cut",
        )?;
        let hold = f.head.gate.holds.values().next().unwrap().clone();
        let c = hold.continuation.as_ref().unwrap();
        let original = f.head.clone();
        f.head.data_pack = c.data_pack;
        f.head.data_end = c.data_end;
        f.head.meta_pack = c.meta_pack;
        f.head.meta_end = c.meta_end;
        let loc = f
            .lookup(&c.active, c.inventory.active.as_ref().unwrap().offset)?
            .unwrap();
        f.head = original.clone();
        let mut file = OpenOptions::new()
            .write(true)
            .open(f.data.path(loc.data.pack))
            .map_err(error)?;
        file.seek(SeekFrom::Start(loc.data.offset)).map_err(error)?;
        file.write_all(b"!").map_err(error)?;
        file.sync_all().map_err(error)?;
        f = Fixture::reopen_combined(&f.dir)?;
        ensure(
            f.resume_checkpoint(&mut source, 0, &hold)
                .unwrap_err()
                .contains("digest"),
            "corrupt ownership must fail content verification",
        )?;
        ensure(
            f.head == original
                && f.head.gate.holds.get(&hold.token) == Some(&hold)
                && f.dir.join("intent").exists(),
            "corruption released original evidence",
        )
    }
    #[test]
    fn every_pin_keeps_its_inventory_and_unclaimed_metadata_can_relocate() -> Result<()> {
        let (_temp, _source, mut f) = setup(512, Kind::Radix)?;
        let claim = Claim::capture(&f, true, 0)?;
        publish_claim(&mut f, &claim, Cut::None)?;
        let mut pins = vec![];
        for class in EXTRA_CLASSES {
            pins.push(f.acquire(class, class == PinClass::UnresolvedIntent)?);
        }
        let another = Claim::capture(&f, true, f.data.pack)?;
        publish_claim(&mut f, &another, Cut::None)?;
        f.audit_combined()?;
        ensure(
            f.gate_candidate(0, true).is_err(),
            "claimed data candidate admitted",
        )?;
        for pin in &pins {
            ensure(
                pin.inventory.active.is_some(),
                "pin dropped ownership commitment",
            )?;
            f.release(pin, true, true, true)?;
        }
        let semantic = f.head.semantic.clone();
        let inventory = f.head.inventory.clone();
        ensure(f.meta.pack > 0, "sealed metadata fixture")?;
        f.compact_combined(false, 0, Fault::None)?;
        f.audit_combined()?;
        ensure(
            f.head.semantic == semantic && f.head.inventory == inventory,
            "physical relocation changed semantic/ownership identities",
        )
    }
    #[test]
    fn explicit_import_is_read_only_until_full_typed_audit() -> Result<()> {
        let (_temp, _source, mut f) = setup(64, Kind::Btree)?;
        let claim = Claim::capture(&f, true, 0)?;
        publish_claim(&mut f, &claim, Cut::None)?;
        f = Fixture::reopen_combined(&f.dir)?;
        f.imported_read_only = true;
        let before = f.snapshot()?;
        f.structural_probe()?;
        ensure(
            publish_claim(&mut f, &claim, Cut::None).is_err() && f.snapshot()? == before,
            "unaudited imported mutation",
        )?;
        f.audit_combined()?;
        publish_claim(&mut f, &claim, Cut::None)?;
        f.audit_combined()?;
        Ok(())
    }
}
