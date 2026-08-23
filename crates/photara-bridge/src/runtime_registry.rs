use std::{collections::BTreeMap, sync::Arc};

use photara_core::{
    CanonicalDigest, NodeDefinitionRef, NodeEvaluationOutput, NodeEvaluationRequest,
    NodeExecutionError, NodeRuntime,
};
use photara_node_sdk::NodePackageManifest;
use thiserror::Error;

/// Application-host registry mapping exact installed definition coordinates to
/// their runtime implementations.
#[derive(Clone, Default)]
pub(crate) struct RuntimeRegistry {
    runtimes: BTreeMap<NodeDefinitionRef, Arc<dyn NodeRuntime + Send + Sync>>,
}

impl RuntimeRegistry {
    pub(crate) fn register_manifest<R>(
        &mut self,
        manifest: &NodePackageManifest,
        runtime: R,
    ) -> Result<(), RuntimeRegistryError>
    where
        R: NodeRuntime + Send + Sync + 'static,
    {
        let runtime: Arc<dyn NodeRuntime + Send + Sync> = Arc::new(runtime);
        let references = manifest
            .definitions
            .iter()
            .map(|definition| NodeDefinitionRef {
                package_id: manifest.package_id.clone(),
                package_version: manifest.package_version.clone(),
                definition_id: definition.id.clone(),
                definition_version: definition.version,
            })
            .collect::<Vec<_>>();
        for reference in &references {
            if runtime.implementation_fingerprint(reference).is_none() {
                return Err(RuntimeRegistryError::UnsupportedDefinition(
                    reference.clone(),
                ));
            }
            if self.runtimes.contains_key(reference) {
                return Err(RuntimeRegistryError::AlreadyRegistered(reference.clone()));
            }
        }
        self.runtimes.extend(
            references
                .into_iter()
                .map(|reference| (reference, Arc::clone(&runtime))),
        );
        Ok(())
    }
}

impl NodeRuntime for RuntimeRegistry {
    fn implementation_fingerprint(
        &self,
        definition: &NodeDefinitionRef,
    ) -> Option<CanonicalDigest> {
        self.runtimes
            .get(definition)?
            .implementation_fingerprint(definition)
    }

    fn evaluate(
        &self,
        request: NodeEvaluationRequest,
    ) -> Result<NodeEvaluationOutput, NodeExecutionError> {
        let definition = request.node.definition.clone();
        let runtime = self
            .runtimes
            .get(&definition)
            .ok_or_else(|| NodeExecutionError {
                code: "photara.runtime.unavailable".to_owned(),
                message: format!("no runtime is registered for exact definition {definition:?}"),
            })?;
        runtime.evaluate(request)
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub(crate) enum RuntimeRegistryError {
    #[error("runtime does not support exact definition {0:?}")]
    UnsupportedDefinition(NodeDefinitionRef),
    #[error("runtime already registered for exact definition {0:?}")]
    AlreadyRegistered(NodeDefinitionRef),
}

#[cfg(test)]
mod tests {
    use photara_asset_set_node::{AssetSetNodePackage, AssetSetNodeRuntime};
    use photara_node_sdk::NodePackage;

    use super::*;

    #[test]
    fn registration_is_exact_and_rejects_duplicate_ownership() {
        let manifest = AssetSetNodePackage.manifest();
        let mut registry = RuntimeRegistry::default();
        registry
            .register_manifest(&manifest, AssetSetNodeRuntime)
            .unwrap();
        let definition = NodeDefinitionRef {
            package_id: manifest.package_id.clone(),
            package_version: manifest.package_version.clone(),
            definition_id: manifest.definitions[0].id.clone(),
            definition_version: manifest.definitions[0].version,
        };
        assert!(registry.implementation_fingerprint(&definition).is_some());
        assert_eq!(
            registry.register_manifest(&manifest, AssetSetNodeRuntime),
            Err(RuntimeRegistryError::AlreadyRegistered(definition))
        );
    }
}
