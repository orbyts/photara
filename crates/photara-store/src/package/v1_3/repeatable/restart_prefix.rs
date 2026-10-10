//! Incremental hashes belonging to one borrowed, immutable current snapshot.
//! Callers' bytes must still match that snapshot; foreign/shorter inputs use the
//! ordinary hash path. No filesystem observation or authority is memoized here.
use super::{OriginalPackage, wire};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::BTreeMap};

pub(in super::super) fn allowance(maximum: usize) -> usize {
    maximum / 64
}
struct Prefix {
    length: usize,
    state: Sha256,
    digest: String,
}
pub(in super::super) struct Prefixes<'a> {
    source: &'a OriginalPackage,
    entries: RefCell<BTreeMap<String, Prefix>>,
    maximum: std::cell::Cell<usize>,
    bytes: RefCell<usize>,
    #[cfg(test)]
    pub(in super::super) hashed: std::cell::Cell<usize>,
}
impl<'a> Prefixes<'a> {
    pub(in super::super) fn new(source: &'a OriginalPackage, maximum: usize) -> Self {
        Self {
            source,
            entries: RefCell::new(BTreeMap::new()),
            maximum: std::cell::Cell::new(allowance(maximum)),
            bytes: RefCell::new(0),
            #[cfg(test)]
            hashed: std::cell::Cell::new(0),
        }
    }
    pub(in super::super) fn limit(&self, maximum: usize) {
        let maximum = allowance(maximum);
        self.maximum.set(maximum);
        if *self.bytes.borrow() > maximum {
            self.entries.borrow_mut().clear();
            *self.bytes.borrow_mut() = 0;
        }
    }
    pub(in super::super) fn hash(&self, id: &str, bytes: &[u8]) -> String {
        let fallback = || {
            #[cfg(test)]
            self.hashed.set(self.hashed.get() + bytes.len());
            wire::hash(bytes)
        };
        if !self
            .source
            .allocations
            .get(id)
            .is_some_and(|a| a.bytes.starts_with(bytes))
        {
            return fallback();
        }
        let mut entries = self.entries.borrow_mut();
        if let Some(prefix) = entries.get_mut(id) {
            if bytes.len() < prefix.length {
                return fallback();
            }
            if bytes.len() > prefix.length {
                #[cfg(test)]
                self.hashed
                    .set(self.hashed.get() + bytes.len() - prefix.length);
                prefix.state.update(&bytes[prefix.length..]);
                prefix.length = bytes.len();
                prefix.digest = format!("{:x}", prefix.state.clone().finalize());
            }
            return prefix.digest.clone();
        }
        // Includes SHA state, owned key/digest, map node and allocator overhead.
        let cost = 512usize.saturating_add(id.len());
        if self.bytes.borrow().saturating_add(cost) > self.maximum.get() {
            return fallback();
        }
        #[cfg(test)]
        self.hashed.set(self.hashed.get() + bytes.len());
        let state = Sha256::new_with_prefix(bytes);
        let digest = format!("{:x}", state.clone().finalize());
        entries.insert(
            id.to_owned(),
            Prefix {
                length: bytes.len(),
                state,
                digest: digest.clone(),
            },
        );
        *self.bytes.borrow_mut() += cost;
        digest
    }
}
