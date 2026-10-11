//! Private allocation ownership; public Vec DTOs remain unchanged.
//!
//! Prefix views expose only their checked extent. Mutation first detaches that
//! visible prefix, so hidden suffixes never become writable or reappear on growth.
//! Historical memo entries must own compact suffixes, not retain these full-pack
//! backing allocations. Sharing does not replace byte or observation validation.
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};
#[cfg(test)]
thread_local! { static COPIED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
fn copied(n: usize) {
    #[cfg(test)]
    COPIED.with(|v| v.set(v.get().saturating_add(n)));
    #[cfg(not(test))]
    let _ = n;
}
#[cfg(test)]
pub(super) fn take_copied() -> usize {
    COPIED.with(|v| v.replace(0))
}
#[derive(Clone, Debug)]
pub(crate) struct Bytes {
    backing: Arc<Vec<u8>>,
    end: usize,
}
impl Bytes {
    #[cfg(test)]
    pub(super) fn shares_backing(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.backing, &other.backing)
    }
    pub(crate) fn prefix(&self, end: usize) -> Option<Self> {
        (end <= self.end).then(|| Self {
            backing: self.backing.clone(),
            end,
        })
    }
    fn owned(&mut self) -> &mut Vec<u8> {
        if self.end != self.backing.len() {
            copied(self.end);
            self.backing = Arc::new(self.backing[..self.end].to_vec());
        } else if Arc::strong_count(&self.backing) > 1 {
            copied(self.end);
        }
        Arc::make_mut(&mut self.backing)
    }
    #[cfg(all(target_os = "macos", any(test, feature = "controlled-disposable")))]
    pub(crate) fn into_vec(self) -> Vec<u8> {
        match Arc::try_unwrap(self.backing) {
            Ok(mut bytes) => {
                bytes.truncate(self.end);
                bytes
            }
            Err(bytes) => {
                copied(self.end);
                bytes[..self.end].to_vec()
            }
        }
    }
    pub(crate) fn as_slice(&self) -> &[u8] {
        self
    }
    pub(crate) fn capacity(&self) -> usize {
        self.backing.capacity()
    }
    pub(crate) fn to_vec(&self) -> Vec<u8> {
        copied(self.end);
        self.as_slice().to_vec()
    }
    pub(crate) fn extend<T: std::borrow::Borrow<u8>>(
        &mut self,
        bytes: impl IntoIterator<Item = T>,
    ) {
        self.owned().extend(bytes.into_iter().map(|b| *b.borrow()));
        self.end = self.backing.len();
    }
    #[cfg(test)]
    pub(crate) fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.owned().extend_from_slice(bytes);
        self.end = self.backing.len();
    }
    pub(crate) fn resize(&mut self, n: usize, value: u8) {
        self.owned().resize(n, value);
        self.end = n;
    }
    #[cfg(test)]
    pub(crate) fn push(&mut self, value: u8) {
        self.extend([value]);
    }
}
impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Self {
        let end = bytes.len();
        Self {
            backing: Arc::new(bytes),
            end,
        }
    }
}
impl Default for Bytes {
    fn default() -> Self {
        Vec::new().into()
    }
}
impl Deref for Bytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.backing[..self.end]
    }
}
impl DerefMut for Bytes {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.owned().as_mut_slice()
    }
}
impl PartialEq for Bytes {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl Eq for Bytes {}
impl PartialEq<Vec<u8>> for Bytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_slice() == other
    }
}
impl PartialEq<Bytes> for Vec<u8> {
    fn eq(&self, other: &Bytes) -> bool {
        self == other.as_slice()
    }
}
impl Serialize for Bytes {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.as_slice().serialize(s)
    }
}
#[derive(Clone)]
pub(crate) struct Allocation {
    pub bytes: Bytes,
    pub arena: String,
    pub witness: Value,
}
#[derive(Clone)]
pub(crate) struct OriginalPackage {
    pub manifest: Value,
    pub head: Value,
    pub commit: Value,
    pub loose: BTreeMap<String, Vec<u8>>,
    pub allocations: BTreeMap<String, Allocation>,
}
impl From<super::input::OriginalPackage> for OriginalPackage {
    fn from(p: super::input::OriginalPackage) -> Self {
        Self {
            manifest: p.manifest,
            head: p.head,
            commit: p.commit,
            loose: p.loose,
            allocations: p
                .allocations
                .into_iter()
                .map(|(id, a)| {
                    (
                        id,
                        Allocation {
                            bytes: a.bytes.into(),
                            arena: a.arena,
                            witness: a.witness,
                        },
                    )
                })
                .collect(),
        }
    }
}
impl OriginalPackage {
    pub(crate) fn public(&self) -> super::input::OriginalPackage {
        super::input::OriginalPackage {
            manifest: self.manifest.clone(),
            head: self.head.clone(),
            commit: self.commit.clone(),
            loose: self.loose.clone(),
            allocations: self
                .allocations
                .iter()
                .map(|(id, a)| {
                    (
                        id.clone(),
                        super::input::Allocation {
                            bytes: a.bytes.to_vec(),
                            arena: a.arena.clone(),
                            witness: a.witness.clone(),
                        },
                    )
                })
                .collect(),
        }
    }
    pub(crate) fn loose(&self, r: &Value) -> super::wire::Result<Value> {
        let k = super::wire::key(r)?;
        let bytes = self.loose.get(&k.0).ok_or("missing original control")?;
        super::wire::ensure(
            bytes.len() as u64 == k.1 && super::wire::hash(bytes) == k.0,
            "original control commitment",
        )?;
        super::wire::parse(bytes)
    }
}
