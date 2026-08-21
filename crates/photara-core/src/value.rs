use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{SchemaId, SchemaVersion, ValueTypeId, ValueTypeVersion};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SchemaRef {
    pub id: SchemaId,
    pub version: SchemaVersion,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ValueTypeRef {
    pub id: ValueTypeId,
    pub version: ValueTypeVersion,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValueTypeDescriptor {
    pub value_type: ValueTypeRef,
    pub display_name: String,
    pub schema: SchemaRef,
}

/// A value carrying the exact semantic type expected by a port.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TypedValue {
    pub value_type: ValueTypeRef,
    pub value: Value,
}

/// Persisted node-owned data carrying its exact schema identity and version.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SchemaValue {
    pub schema: SchemaRef,
    pub value: Value,
}

/// The minimum registry needed to resolve and compare typed values and ports.
#[derive(Clone, Debug, Default)]
pub struct ValueTypeRegistry {
    descriptors: BTreeMap<ValueTypeRef, ValueTypeDescriptor>,
}

impl ValueTypeRegistry {
    /// Registers one exact value-type version.
    ///
    /// # Errors
    ///
    /// Returns [`ValueTypeRegistryError::AlreadyRegistered`] for a duplicate
    /// identity or [`ValueTypeRegistryError::EmptyDisplayName`] for a blank name.
    pub fn register(
        &mut self,
        descriptor: ValueTypeDescriptor,
    ) -> Result<(), ValueTypeRegistryError> {
        if descriptor.display_name.trim().is_empty() {
            return Err(ValueTypeRegistryError::EmptyDisplayName(
                descriptor.value_type,
            ));
        }
        if self.descriptors.contains_key(&descriptor.value_type) {
            return Err(ValueTypeRegistryError::AlreadyRegistered(
                descriptor.value_type,
            ));
        }
        self.descriptors
            .insert(descriptor.value_type.clone(), descriptor);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, value_type: &ValueTypeRef) -> Option<&ValueTypeDescriptor> {
        self.descriptors.get(value_type)
    }

    #[must_use]
    pub fn are_directly_compatible(&self, output: &ValueTypeRef, input: &ValueTypeRef) -> bool {
        output == input && self.descriptors.contains_key(output)
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ValueTypeRegistryError {
    #[error("value type {0:?} is already registered")]
    AlreadyRegistered(ValueTypeRef),
    #[error("value type {0:?} has an empty display name")]
    EmptyDisplayName(ValueTypeRef),
}
