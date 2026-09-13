// Integration fixtures intentionally exercise the complete public contract surface.
#![allow(clippy::wildcard_imports)]
use photara_core::{
    PackageVersion,
    contracts::{access::*, asset_set::*, ids::*, resource::*, schema::*},
};
use photara_node_sdk::v2::*;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
fn q(s: &str) -> QualifiedName {
    QualifiedName::parse(s).unwrap()
}
fn n(s: &str) -> LocalName {
    LocalName::parse(s).unwrap()
}
fn schema(id: &str) -> SchemaRef {
    SchemaRef {
        id: q(id),
        version: Version::FIRST,
    }
}
fn value(id: &str, version: u32) -> ValueTypeRef {
    ValueTypeRef {
        id: q(id),
        version: Version::new(version).unwrap(),
    }
}
fn runtime() -> RuntimeCoordinate {
    RuntimeCoordinate {
        runtime_id: q("example.runtime"),
        contract_version: Version::FIRST,
    }
}
fn digest(n: u8) -> Digest {
    Digest::from_bytes([n; 32])
}
fn spec() -> ManifestSpec {
    let coordinate = DefinitionCoordinate {
        package_id: q("example.review"),
        package_version: PackageVersion::new(1, 2, 3),
        definition_id: q("example.review.inspect"),
        definition_version: Version::new(3).unwrap(),
    };
    ManifestSpec {
        manifest_schema_version: 2,
        contract_version: Version::FIRST,
        package_id: coordinate.package_id.clone(),
        package_version: coordinate.package_version.clone(),
        display_name: "Review".into(),
        required_features: vec![
            RequiredFeature::TypedPorts,
            RequiredFeature::ContextDeclarations,
            RequiredFeature::AssetSetV2,
            RequiredFeature::Inspector,
        ],
        definitions: vec![NodeDefinitionV2 {
            coordinate,
            ports: vec![PortContract {
                id: n("assets"),
                direction: PortDirection::Input,
                cardinality: PortCardinality::One,
                value_type: value("photara.asset-set", 2),
                schema: schema("photara.value.asset-set-snapshot"),
                family: ValueFamily::AssetSet,
            }],
            configuration_schema: AuthoredSchema {
                schema: schema("example.review.config"),
                fields: vec![FieldContract {
                    name: n("label"),
                    mode: FieldMode::LiteralOnly,
                    visibility: AuthoredVisibility::Public,
                    shape: ValueShape::Leaf {
                        value_type: value("example.text", 1),
                        family: ValueFamily::Configuration,
                    },
                }],
            },
            state_schema: None,
            execution: ExecutionContract::Pure,
            capabilities: vec![],
            determinism: Determinism::Deterministic,
            cache: CacheContract {
                scope: CacheScope::Project,
                evaluation_key_version: Version::new(2).unwrap(),
                verified_capture_required: false,
            },
            context: ContextDeclaration {
                contract_version: Version::FIRST,
                expression_version: Version::FIRST,
                captured_facts: vec![],
            },
            inspector: InspectorContract {
                schema: schema("photara.inspector"),
                contribution: Contribution {
                    id: q("photara.inspector.generic"),
                    contract_version: Version::FIRST,
                },
                sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::Ports,
                    InspectorSection::Parameters,
                    InspectorSection::StateSummaries,
                    InspectorSection::Effects,
                    InspectorSection::Diagnostics,
                ],
            },
            work_surface: None,
            presentation: PresentationContract {
                brand: "Example".into(),
                display_name: "Review".into(),
                icon_resource: q("example.review.icon"),
                taxonomy_version: Version::FIRST,
                primary_category: q("photara.category.inspection.review"),
                search_terms: vec!["review".into()],
                provider_tags: vec![],
                capability_tags: vec![],
                activation: None,
            },
            platforms: vec![HostKind::Macos, HostKind::Windows, HostKind::Linux]
                .into_iter()
                .map(|platform| PlatformContract {
                    platform,
                    runtimes: vec![runtime()],
                    tools: vec![],
                })
                .collect(),
            versions: VersionAxes {
                contract_version: Version::FIRST,
                implementation_digest: digest(7),
                context_version: Version::FIRST,
                expression_version: Version::FIRST,
                evaluation_key_version: Version::new(2).unwrap(),
            },
            migrations: vec![],
        }],
    }
}
fn registry() -> ContractRegistry {
    let mut r = ContractRegistry::default();
    for s in [
        schema("example.review.config"),
        schema("photara.inspector"),
        schema("photara.value.asset-set-snapshot"),
        schema("example.text.schema"),
        schema("example.query.result"),
        schema("example.effect.request"),
        schema("example.effect.receipt"),
        schema("example.migration.diagnostics"),
    ] {
        r.register_schema(s).unwrap();
    }
    r.register_value(
        value("photara.asset-set", 2),
        schema("photara.value.asset-set-snapshot"),
        ValueFamily::AssetSet,
    )
    .unwrap();
    r.register_value(
        value("example.text", 1),
        schema("example.text.schema"),
        ValueFamily::Configuration,
    )
    .unwrap();
    r
}
fn host() -> HostAvailability {
    HostAvailability {
        platform: HostKind::Linux,
        installed_implementation: Some(digest(7)),
        trusted: true,
        runtimes: BTreeSet::from([runtime()]),
        tools: BTreeSet::new(),
        native_skin_available: true,
    }
}
fn captured(d: &mut NodeDefinitionV2) {
    d.capabilities.push(CapabilityRequirement::Selector {
        slot: n("input"),
        required: true,
        scope: SelectorScope::InputPort {
            port_id: n("assets"),
        },
        access: AccessMode::Captured,
        actions: ProjectPreset::Reader.mask(),
        value_type: value("example.text", 1),
        fields: vec![n("content_digest")],
        max_items: 1,
    });
    d.context.captured_facts = vec![CapturedFact {
        id: n("input_digest"),
        value_type: value("example.text", 1),
        source_slot: n("input"),
    }];
    d.determinism = Determinism::Captured {
        facts: vec![n("input_digest")],
    };
    d.cache.verified_capture_required = true;
}
fn effect() -> NodeDefinitionV2 {
    let mut d = spec().definitions.remove(0);
    d.cache.scope = CacheScope::None;
    d.determinism = Determinism::NonDeterministic;
    d.capabilities = vec![CapabilityRequirement::Resource {
        slot: n("target"),
        required: true,
        access: AccessMode::Live,
        storage_class: StorageClass::ExternalOutput,
        rights: ResourceRights::new(vec![ResourceRight::Create]).unwrap(),
        subtree: Some(RelativeComponents::new(vec!["exports".into()]).unwrap()),
        max_items: 10,
        max_bytes: DecimalU64::new(1000),
        operations: vec![n("write")],
    }];
    d.execution = ExecutionContract::Effect {
        operations: vec![EffectOperation {
            id: n("write"),
            kind: q("example.write"),
            target_slot: n("target"),
            credential_slot: None,
            rights: ResourceRights::new(vec![ResourceRight::Create]).unwrap(),
            request_schema: schema("example.effect.request"),
            receipt_schema: schema("example.effect.receipt"),
            replacement_supported: false,
        }],
    };
    d
}
fn work() -> WorkSurfaceContract {
    WorkSurfaceContract {
        contribution: Contribution {
            id: q("example.review.surface"),
            contract_version: Version::FIRST,
        },
        components: vec![HostComponentRequest {
            instance_id: n("browser"),
            component_id: HostComponentId::Assets,
            contract_version: Version::FIRST,
            input: ComponentBinding::AssetPort {
                port_id: n("assets"),
            },
            actions: vec![ComponentAction::Inspect, ComponentAction::Select],
            result_schema: schema("example.query.result"),
        }],
    }
}
fn migration(d: &NodeDefinitionV2) -> MigrationDeclaration {
    let mut from = d.coordinate.clone();
    from.package_version = PackageVersion::new(1, 0, 0);
    from.definition_version = Version::new(2).unwrap();
    MigrationDeclaration {
        migration_id: q("example.review.upgrade"),
        from,
        to: d.coordinate.clone(),
        from_config: d.configuration_schema.schema.clone(),
        to_config: d.configuration_schema.schema.clone(),
        from_state: None,
        to_state: None,
        deterministic: true,
        implementation_digest: digest(8),
        id_remap: IdRemapContract::NoIdBearingState,
        diagnostics_schema: schema("example.migration.diagnostics"),
    }
}
#[test]
fn complete_manifest_round_trips_and_requires_exact_registered_schemas() {
    let m = NodePackageManifestV2::try_from(spec()).unwrap();
    m.validate_registered(&registry()).unwrap();
    let bytes = m.canonical_bytes().unwrap();
    assert_eq!(NodePackageManifestV2::from_json(&bytes).unwrap(), m);
    assert!(m.validate_registered(&ContractRegistry::default()).is_err());
}
#[test]
fn versions_and_coordinates_are_independent_and_exact() {
    let base = spec();
    for index in 0..7 {
        let mut s = base.clone();
        match index {
            0 => s.manifest_schema_version = 1,
            1 => s.contract_version = Version::new(2).unwrap(),
            2 => s.definitions[0].coordinate.package_version = PackageVersion::new(9, 0, 0),
            3 => s.definitions[0].coordinate.definition_id = q("other.node"),
            4 => s.definitions[0].versions.evaluation_key_version = Version::FIRST,
            5 => s.definitions[0].context.expression_version = Version::new(2).unwrap(),
            _ => s.definitions[0].versions.context_version = Version::new(2).unwrap(),
        }
        assert!(NodePackageManifestV2::try_from(s).is_err());
    }
    let mut s = base;
    s.definitions[0].coordinate.definition_version = Version::new(99).unwrap();
    s.definitions[0].configuration_schema.schema.version = Version::new(7).unwrap();
    NodePackageManifestV2::try_from(s).unwrap();
}
#[test]
fn manifest_unknown_required_semantics_duplicate_keys_and_native_fields_fail_closed() {
    let base = serde_json::to_value(spec()).unwrap();
    for field in [
        "native_skin",
        "host_path",
        "secret",
        "work_surface_contribution_id",
    ] {
        let mut v = base.clone();
        v["definitions"][0]["presentation"][field] = json!("private");
        assert!(NodePackageManifestV2::from_json(&serde_json::to_vec(&v).unwrap()).is_err());
    }
    let mut v = base.clone();
    v["required_features"]
        .as_array_mut()
        .unwrap()
        .push(json!("unknown-feature"));
    assert!(NodePackageManifestV2::from_json(&serde_json::to_vec(&v).unwrap()).is_err());
    let bytes = serde_json::to_string(&base).unwrap();
    let duplicate = bytes.replacen('{', "{\"manifest_schema_version\":2,", 1);
    assert!(NodePackageManifestV2::from_json(duplicate.as_bytes()).is_err());
}
#[test]
fn mandatory_inspector_is_not_an_optional_v1_extension() {
    let mut s = spec();
    s.definitions[0].inspector.sections.pop();
    assert!(NodePackageManifestV2::try_from(s).is_err());
    let mut v = serde_json::to_value(spec()).unwrap();
    let inspector = v["definitions"][0]
        .as_object_mut()
        .unwrap()
        .remove("inspector")
        .unwrap();
    v["definitions"][0]["photara.presentation"] = inspector;
    assert!(NodePackageManifestV2::from_json(&serde_json::to_vec(&v).unwrap()).is_err());
}
#[test]
fn typed_ports_reject_v1_assetsets_fanin_optional_outputs_and_live_families() {
    let base = spec();
    for i in 0..6 {
        let mut s = base.clone();
        let p = &mut s.definitions[0].ports[0];
        match i {
            0 => p.value_type.version = Version::FIRST,
            1 => p.family = ValueFamily::Configuration,
            2 => {
                p.direction = PortDirection::Output;
                p.cardinality = PortCardinality::Optional;
            }
            3 => {
                p.value_type = value("example.secret", 1);
                p.family = ValueFamily::SecretRef;
            }
            4 => p.value_type = value("example.assets", 2),
            _ => {
                let p = p.clone();
                s.definitions[0].ports.push(p);
            }
        }
        assert!(NodePackageManifestV2::try_from(s).is_err());
    }
    let mut v = serde_json::to_value(base).unwrap();
    v["definitions"][0]["ports"][0]["cardinality"] = json!("many");
    assert!(NodePackageManifestV2::from_json(&serde_json::to_vec(&v).unwrap()).is_err());
}
#[test]
fn schema_registry_rejects_wrong_family_and_version_without_retargeting() {
    let m = NodePackageManifestV2::try_from(spec()).unwrap();
    let mut r = registry();
    assert!(
        r.register_value(
            value("photara.asset-set", 2),
            schema("photara.value.asset-set-snapshot"),
            ValueFamily::Configuration
        )
        .is_err()
    );
    let mut s = spec();
    s.definitions[0].ports[0].schema.version = Version::new(2).unwrap();
    let wrong = NodePackageManifestV2::try_from(s).unwrap();
    assert!(wrong.validate_registered(&r).is_err());
    m.validate_registered(&r).unwrap();
}
#[test]
fn variable_field_modes_do_not_smuggle_workflow_or_secret_families() {
    for mode in [
        FieldMode::LiteralOnly,
        FieldMode::Expression,
        FieldMode::Template,
    ] {
        let mut s = spec();
        s.definitions[0].configuration_schema.fields[0].mode = mode;
        NodePackageManifestV2::try_from(s.clone()).unwrap();
        s.definitions[0].configuration_schema.fields[0].shape = ValueShape::List {
            max_items: 1,
            item: Box::new(ValueShape::Leaf {
                value_type: value("photara.asset-set", 2),
                family: ValueFamily::AssetSet,
            }),
        };
        assert!(NodePackageManifestV2::try_from(s).is_err());
    }
}
#[test]
fn declaration_never_becomes_a_grant_and_pure_never_acquires_live_access() {
    let mut d = spec().definitions.remove(0);
    captured(&mut d);
    d.validate().unwrap();
    if let CapabilityRequirement::Selector { access, .. } = &mut d.capabilities[0] {
        *access = AccessMode::Live;
    }
    assert!(d.validate().is_err());
    let mut d = effect();
    d.execution = ExecutionContract::Read;
    assert!(d.validate().is_err());
    d.execution = ExecutionContract::Pure;
    assert!(d.validate().is_err());
    let mut s = serde_json::to_value(spec()).unwrap();
    s["definitions"][0]["capabilities"] = json!([{"kind":"credential","slot":"login","required":true,"provider_id":"example.provider","operations":["read"],"token":"secret"}]);
    assert!(NodePackageManifestV2::from_json(&serde_json::to_vec(&s).unwrap()).is_err());
}
#[test]
fn cache_matrix_cannot_skip_effects_or_reuse_nondeterministic_results() {
    for execution in 0..3 {
        for determinism in 0..3 {
            for scope in [CacheScope::None, CacheScope::Device, CacheScope::Project] {
                let mut d = if execution == 2 {
                    effect()
                } else {
                    spec().definitions.remove(0)
                };
                if execution == 1 {
                    d.execution = ExecutionContract::Read;
                }
                match determinism {
                    0 => d.determinism = Determinism::Deterministic,
                    1 => captured(&mut d),
                    _ => d.determinism = Determinism::NonDeterministic,
                }
                d.cache.scope = scope;
                let expected = scope == CacheScope::None
                    || (execution != 2 && determinism != 2 && (execution != 1 || determinism == 1));
                assert_eq!(
                    d.validate().is_ok(),
                    expected,
                    "{execution} {determinism} {scope:?}"
                );
            }
        }
    }
}
#[test]
fn captured_reads_need_complete_verified_fact_coordinates() {
    let mut d = spec().definitions.remove(0);
    d.execution = ExecutionContract::Read;
    captured(&mut d);
    d.validate().unwrap();
    d.cache.verified_capture_required = false;
    assert!(d.validate().is_err());
    d.cache.verified_capture_required = true;
    d.context.captured_facts.clear();
    assert!(d.validate().is_err());
}
#[test]
fn effects_require_exact_declared_targets_credentials_and_operation_bounds() {
    let d = effect();
    d.validate().unwrap();
    for i in 0..5 {
        let mut d = d.clone();
        if let ExecutionContract::Effect { operations } = &mut d.execution {
            match i {
                0 => operations.clear(),
                1 => operations[0].target_slot = n("undeclared"),
                2 => operations[0].credential_slot = Some(n("missing")),
                3 => {
                    operations[0].rights =
                        ResourceRights::new(vec![ResourceRight::Replace]).unwrap();
                }
                _ => operations.push(operations[0].clone()),
            }
        }
        assert!(d.validate().is_err());
    }
}
#[test]
fn category_and_brand_do_not_determine_execution_or_ports() {
    let original = spec().definitions.remove(0);
    for category in CATEGORIES {
        let mut d = original.clone();
        d.presentation.primary_category = q(category);
        d.presentation.brand = "Different publisher".into();
        d.validate().unwrap();
        assert_eq!(d.execution, original.execution);
        assert_eq!(d.ports, original.ports);
        assert_eq!(d.capabilities, original.capabilities);
    }
}
#[test]
fn work_surface_is_optional_and_asset_browser_requires_an_explicit_input() {
    let mut s = spec();
    s.definitions[0].work_surface = Some(work());
    assert!(NodePackageManifestV2::try_from(s.clone()).is_err());
    s.required_features.push(RequiredFeature::WorkSurface);
    NodePackageManifestV2::try_from(s.clone()).unwrap();
    s.definitions[0].work_surface.as_mut().unwrap().components[0].input =
        ComponentBinding::AssetPort {
            port_id: n("project_assets"),
        };
    assert!(NodePackageManifestV2::try_from(s.clone()).is_err());
    s.definitions[0].work_surface = None;
    NodePackageManifestV2::try_from(s).unwrap();
}
#[test]
fn all_library_pickers_require_scoped_selectors_and_separate_create_rights() {
    for component_id in [
        HostComponentId::Person,
        HostComponentId::Organization,
        HostComponentId::Location,
        HostComponentId::LocationKind,
    ] {
        let mut d = spec().definitions.remove(0);
        d.capabilities.push(CapabilityRequirement::Selector {
            slot: n("people"),
            required: true,
            scope: SelectorScope::Library,
            access: AccessMode::Captured,
            actions: ProjectPreset::Reader.mask(),
            value_type: value("example.text", 1),
            fields: vec![n("name")],
            max_items: 10,
        });
        let mut w = work();
        w.components[0].component_id = component_id;
        w.components[0].input = ComponentBinding::LibraryQuery {
            selector_slot: n("people"),
        };
        d.work_surface = Some(w);
        d.validate().unwrap();
        d.work_surface.as_mut().unwrap().components[0]
            .actions
            .push(ComponentAction::CreateLibraryRecord);
        assert!(d.validate().is_err());
        if let CapabilityRequirement::Selector { actions, .. } = &mut d.capabilities[0] {
            *actions = ProjectPreset::Author.mask();
        }
        d.validate().unwrap();
    }
}
#[test]
fn trust_runtime_platform_and_native_skin_are_independent() {
    let d = spec().definitions.remove(0);
    let r = registry();
    let base = host();
    let ready = d.availability(&r, &base).unwrap();
    assert!(ready.portable_evaluation);
    assert!(!ready.migration_execution);
    for i in 0..5 {
        let mut h = base.clone();
        let reason = match i {
            0 => {
                h.installed_implementation = None;
                FallbackReason::MissingRuntime
            }
            1 => {
                h.trusted = false;
                FallbackReason::Untrusted
            }
            2 => {
                h.installed_implementation = Some(digest(8));
                FallbackReason::IncompatibleRuntime
            }
            3 => {
                h.runtimes.clear();
                FallbackReason::IncompatibleRuntime
            }
            _ => {
                h.native_skin_available = false;
                FallbackReason::Ready
            }
        };
        let result = d.availability(&r, &h).unwrap();
        assert_eq!(result.reason, reason);
        assert_eq!(result.portable_evaluation, i == 4);
        assert!(!result.node_presentation_code);
    }
    let mut d = d;
    d.platforms.retain(|p| p.platform != HostKind::Linux);
    assert_eq!(
        d.availability(&r, &base).unwrap().reason,
        FallbackReason::UnsupportedPlatform
    );
}
#[test]
fn missing_schema_prevents_evaluation_even_with_trusted_installed_code() {
    let d = spec().definitions.remove(0);
    let result = d
        .availability(&ContractRegistry::default(), &host())
        .unwrap();
    assert_eq!(result.reason, FallbackReason::MissingSchema);
    assert!(!result.semantic_edits);
    assert!(result.host_owned_inspector);
}
#[test]
fn migrations_are_explicit_exact_deterministic_and_never_execute_on_inspection() {
    let mut s = spec();
    let m = migration(&s.definitions[0]);
    s.definitions[0].migrations.push(m.clone());
    let validated = NodePackageManifestV2::try_from(s.clone()).unwrap();
    validated.validate_registered(&registry()).unwrap();
    let inspected = inspect_manifest(&validated.canonical_bytes().unwrap()).unwrap();
    assert!(!inspected.fallback.migration_execution);
    for i in 0..5 {
        let mut bad = s.clone();
        let m = &mut bad.definitions[0].migrations[0];
        match i {
            0 => m.deterministic = false,
            1 => m.to.definition_version = Version::new(99).unwrap(),
            2 => m.to_config = schema("wrong.config"),
            3 => m.from = m.to.clone(),
            _ => m.to_state = Some(schema("wrong.state")),
        }
        assert!(NodePackageManifestV2::try_from(bad).is_err());
    }
}
#[test]
fn fallback_preserves_unknown_and_untrusted_original_bytes_without_running_code() {
    for bytes in [
        br#"{"manifest_schema_version":99,"opaque":{"id":"unchanged"}}"#.as_slice(),
        br#"{"broken":"private-path""#.as_slice(),
    ] {
        let i = inspect_manifest(bytes).unwrap();
        assert_eq!(i.original_bytes(), bytes);
        assert!(!i.fallback.node_presentation_code);
        assert!(!i.fallback.semantic_edits);
        assert!(!i.fallback.portable_evaluation);
        assert!(!i.fallback.migration_execution);
    }
    assert!(inspect_manifest(&vec![b' '; MAX_MANIFEST_BYTES + 1]).is_err());
}
#[test]
fn v1_manifest_stays_inspectable_registrable_and_gets_no_v2_rights() {
    let legacy = photara_node_sdk::NodePackageManifest {
        manifest_schema_version: photara_core::SchemaVersion::first(),
        package_id: photara_core::NodePackageId::parse("example.old").unwrap(),
        package_version: PackageVersion::new(0, 1, 0),
        display_name: "Old".into(),
        definitions: vec![photara_core::NodeDefinition {
            id: photara_core::NodeDefinitionId::parse("example.old.source").unwrap(),
            version: photara_core::NodeDefinitionVersion::first(),
            display_name: "Source".into(),
            ports: vec![],
            config_schema: photara_core::SchemaRef {
                id: photara_core::SchemaId::parse("example.old.config").unwrap(),
                version: photara_core::SchemaVersion::first(),
            },
            authored_state_schema: None,
            capabilities: BTreeSet::new(),
            extensions: BTreeMap::new(),
        }],
        extensions: BTreeMap::from([("future-field".into(), json!({"id":"preserved"}))]),
    };
    let bytes = serde_json::to_vec(&legacy).unwrap();
    let mut registry = photara_node_sdk::NodePackageRegistry::default();
    registry.register_manifest(legacy.clone()).unwrap();
    let i = inspect_manifest(&bytes).unwrap();
    assert_eq!(i.original_bytes(), bytes);
    assert_eq!(i.fallback.reason, FallbackReason::LegacyV1);
    assert!(!i.fallback.portable_evaluation);
    assert!(NodePackageManifestV2::from_json(&bytes).is_err());
    assert_eq!(
        serde_json::from_slice::<photara_node_sdk::NodePackageManifest>(&bytes).unwrap(),
        legacy
    );
}
#[test]
fn duplicate_definitions_features_schema_and_capability_slots_are_rejected() {
    let mut s = spec();
    s.definitions.push(s.definitions[0].clone());
    assert!(NodePackageManifestV2::try_from(s).is_err());
    let mut s = spec();
    s.required_features.push(RequiredFeature::TypedPorts);
    assert!(NodePackageManifestV2::try_from(s).is_err());
    let mut d = effect();
    d.capabilities.push(d.capabilities[0].clone());
    assert!(d.validate().is_err());
    let mut r = registry();
    assert!(r.register_schema(schema("photara.inspector")).is_err());
}
fn golden() -> serde_json::Value {
    let manifest = NodePackageManifestV2::try_from(spec()).unwrap();
    manifest.validate_registered(&registry()).unwrap();
    let project_id: ProjectId = "62000000-0000-4000-8000-000000000001".parse().unwrap();
    let snapshot_id: AssetSetSnapshotId = "62000000-0000-4000-8000-000000000002".parse().unwrap();
    let snapshot = AssetSetSnapshot::new(
        snapshot_id,
        project_id,
        vec![AssetSetMember {
            ordinal: 0,
            asset_id: "62000000-0000-4000-8000-000000000003".parse().unwrap(),
            representations: vec![],
            metadata: vec![],
            missing_facts: vec![MissingFact::NoUsableRepresentation],
        }],
    )
    .unwrap();
    let pages: Vec<_> = snapshot
        .pages(500)
        .unwrap()
        .into_iter()
        .map(|p| json!({"object_ref":p.object_ref().unwrap(),"page":p}))
        .collect();
    json!({"schema":"photara.d19-contracts.v1","codec":"photara.canonical-json.v1","valid_action_masks":(0..=255).filter(|&n|ActionMask::new(n).is_ok()).collect::<Vec<_>>(),"manifest":manifest,"manifest_sha256":Digest::of_bytes(&manifest.canonical_bytes().unwrap()),"asset_set":snapshot,"asset_set_descriptor":snapshot.descriptor().unwrap(),"pages":pages,"relative_components":RelativeComponents::new(vec!["café".into(),"cafe\u{301}.tif".into()]).unwrap()})
}
fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/fixtures/generation-two/d19-contracts.json")
}
#[test]
fn rust_generated_d19_golden_vector_matches() {
    let bytes = std::fs::read(fixture_path()).expect("generate the additive D19 fixture first");
    let actual: serde_json::Value = decode_strict(&bytes, MAX_MANIFEST_BYTES).unwrap();
    assert_eq!(actual, golden());
    assert_eq!(bytes, photara_core::canonical_json(&golden()).unwrap());
}
#[test]
#[ignore = "explicit generation only; writes the new D19 fixture, never existing S6 vectors"]
fn generate_additive_d19_fixture() {
    std::fs::write(
        fixture_path(),
        photara_core::canonical_json(&golden()).unwrap(),
    )
    .unwrap();
}

#[test]
fn v2_connections_match_schema_and_semantic_type_without_v1_coercion() {
    let mut input = spec().definitions[0].ports[0].clone();
    let mut output = input.clone();
    output.direction = PortDirection::Output;
    validate_port_connection_v2(&registry(), &output, &input).unwrap();
    input.cardinality = PortCardinality::Optional;
    validate_port_connection_v2(&registry(), &output, &input).unwrap();
    input.schema.version = Version::new(2).unwrap();
    assert!(validate_port_connection_v2(&registry(), &output, &input).is_err());
    input.schema = output.schema.clone();
    input.value_type.version = Version::FIRST;
    assert!(validate_port_connection_v2(&registry(), &output, &input).is_err());
    for (id, family) in [
        ("photara.secret-ref", ValueFamily::SecretRef),
        ("photara.materialized-handle", ValueFamily::LiveHandle),
        (
            "photara.workflow-output-reference",
            ValueFamily::WorkflowOutputReference,
        ),
        ("photara.asset-set", ValueFamily::AssetSet),
    ] {
        let mut registry = ContractRegistry::default();
        registry.register_schema(output.schema.clone()).unwrap();
        output.value_type = value(id, 1);
        output.family = family;
        registry
            .register_value(output.value_type.clone(), output.schema.clone(), family)
            .unwrap();
        input = output.clone();
        input.direction = PortDirection::Input;
        assert!(validate_port_connection_v2(&registry, &output, &input).is_err());
    }
}
