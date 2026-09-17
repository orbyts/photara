//! Disposable executable semantic model. Nothing here is a package codec, a
//! production feature identifier, a storage implementation or durability proof.
//! All object IDs, publication barriers and stores below exist only in memory.
//! The real 1.1 reader remains covered separately by `retention_boundary`.
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod resources;
mod roots;

fn digest(value: &impl Serialize) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Refusal {
    Integrity,
    Unsupported,
    Conflict,
    Reconcile,
    Capacity,
    Unqualified,
    Unstable,
}

type Check<T> = Result<T, Refusal>;
