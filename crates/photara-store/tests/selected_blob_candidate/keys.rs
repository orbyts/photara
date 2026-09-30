//! Mixed identities only at inventory/locator boundaries; existing JSON keys stay tuples.
use photara_store::package::{ObjectKind, ObjectRef};
use serde_json::{Value, json};
pub(crate) type JsonKey = (String, u64);
pub(crate) type Result<T> = std::result::Result<T, &'static str>;
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct MixedObjectKey(ObjectRef);
impl MixedObjectKey {
    pub(crate) fn parse(value: &Value) -> Result<Self> {
        let r: ObjectRef = serde_json::from_value(value.clone()).map_err(|_| "mixed ObjectRef")?;
        if r.kind == ObjectKind::Json && r.byte_length.get() == 0 {
            return Err("empty JSON ObjectRef");
        }
        Ok(Self(r))
    }
    pub(crate) fn from_json_key(key: &JsonKey) -> Result<Self> {
        Self::parse(&json!({"kind":"json","sha256":key.0,"byte_length":key.1.to_string()}))
    }
    pub(crate) fn json_key(&self) -> Result<JsonKey> {
        if self.0.kind != ObjectKind::Json {
            return Err("Blob cannot become JSON key");
        }
        Ok((self.0.sha256.as_str().into(), self.0.byte_length.get()))
    }
    pub(crate) fn object_ref(&self) -> &ObjectRef {
        &self.0
    }
    pub(crate) fn value(&self) -> Result<Value> {
        serde_json::to_value(&self.0).map_err(|_| "mixed reference serialization")
    }
}
/// Tree comparison has an explicit key domain, never a tagged/hash-looking string.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum TreeKey {
    Object(MixedObjectKey),
    Id(uuid::Uuid),
    Ordinal(u64),
}
impl TreeKey {
    pub(crate) fn object(value: &Value) -> Result<Self> {
        Ok(Self::Object(MixedObjectKey::parse(value)?))
    }
    pub(crate) fn id(value: &Value) -> Result<Self> {
        let s = value.as_str().ok_or("tree UUID string")?;
        let id = uuid::Uuid::parse_str(s).map_err(|_| "tree UUID")?;
        if id.is_nil() || id.to_string() != s {
            return Err("tree canonical UUID");
        }
        Ok(Self::Id(id))
    }
    pub(crate) fn ordinal(value: &Value) -> Result<Self> {
        let s = value.as_str().ok_or("tree ordinal string")?;
        let n = s.parse::<u64>().map_err(|_| "tree ordinal range")?;
        if n.to_string() != s {
            return Err("tree canonical ordinal");
        }
        Ok(Self::Ordinal(n))
    }
}
