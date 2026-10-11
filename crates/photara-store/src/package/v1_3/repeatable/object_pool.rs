//! Canonically checked original bytes, materialized only when current planning follows them.
use super::{canonical_memo::Summary, wire};
use serde_json::Value;
use std::{cell::OnceCell, collections::BTreeMap};
#[derive(Debug)]
enum Stored {
    Original {
        raw: Vec<u8>,
        summary: Summary,
        value: OnceCell<Option<Value>>,
    },
    New(Value),
}
#[derive(Default, Debug)]
pub(super) struct ObjectPool {
    entries: BTreeMap<wire::Key, Stored>,
}
impl ObjectPool {
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(super) fn contains_key(&self, k: &wire::Key) -> bool {
        self.entries.contains_key(k)
    }
    pub(super) fn keys(&self) -> impl Iterator<Item = &wire::Key> {
        self.entries.keys()
    }
    pub(super) fn get(&self, k: &wire::Key) -> Option<&Value> {
        match self.entries.get(k)? {
            Stored::New(v) => Some(v),
            Stored::Original { raw, value, .. } => value
                .get_or_init(|| {
                    super::layout::count_decode();
                    // Canonical syntax and every parser bound already checked for these exact bytes.
                    serde_json::from_slice(raw).ok()
                })
                .as_ref(),
        }
    }
    pub(super) fn original(&mut self, k: wire::Key, raw: &[u8], summary: Summary) {
        self.entries.insert(
            k,
            Stored::Original {
                raw: raw.to_vec(),
                summary,
                value: OnceCell::new(),
            },
        );
    }
    pub(super) fn insert(&mut self, k: wire::Key, v: Value) {
        self.entries.insert(k, Stored::New(v));
    }
    pub(super) fn collision(&self, id: &str, root: bool) -> bool {
        self.entries.values().any(|entry| match entry {
            Stored::Original { summary, .. } => {
                if root {
                    summary.root.as_deref() == Some(id)
                } else {
                    summary.association.as_deref() == Some(id)
                }
            }
            Stored::New(v) => {
                if root {
                    v["schema"]["id"] == "photara.package.state-root" && v["root_id"] == id
                } else {
                    v["association_id"] == id
                }
            }
        })
    }
}
impl FromIterator<(wire::Key, Value)> for ObjectPool {
    fn from_iter<T: IntoIterator<Item = (wire::Key, Value)>>(iter: T) -> Self {
        let mut pool = Self::default();
        pool.extend(iter);
        pool
    }
}
impl Extend<(wire::Key, Value)> for ObjectPool {
    fn extend<T: IntoIterator<Item = (wire::Key, Value)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}
impl std::ops::Index<&wire::Key> for ObjectPool {
    type Output = Value;
    fn index(&self, k: &wire::Key) -> &Value {
        self.get(k).expect("checked object key")
    }
}
#[cfg(test)]
impl PartialEq for ObjectPool {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.keys().all(|k| self.get(k) == other.get(k))
    }
}
