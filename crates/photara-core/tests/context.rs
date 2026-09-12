// Contract fixtures deliberately exercise the complete additive public surface.
#![allow(clippy::wildcard_imports)]
use photara_core::context::snapshot::SnapshotSpec;
use photara_core::context::{
    cache::*, expression::*, metadata::*, proposal::*, snapshot::*, value::*, variable::*, *,
};
use photara_core::contracts::{
    access::Principal,
    asset_set::*,
    dto::{AuthorizationGeneration, RevisionCoordinate, ScopeRef},
    ids::*,
    resource::{HostPlace, ResourceDescriptor},
    schema::{
        DecimalU64, Digest, LocalName, LocalRevision, ObjectKind, ObjectRef, QualifiedName, Version,
    },
};
use serde_json::json;
use std::{fmt::Debug, str::FromStr};
fn id<T: FromStr>(n: u64) -> T
where
    T::Err: Debug,
{
    format!("62000000-0000-4000-8000-{n:012}").parse().unwrap()
}
fn n(s: &str) -> LocalName {
    LocalName::parse(s).unwrap()
}
fn q(s: &str) -> QualifiedName {
    QualifiedName::parse(s).unwrap()
}
fn rev() -> RevisionCoordinate {
    RevisionCoordinate::Local {
        revision: LocalRevision::INITIAL,
    }
}
fn time() -> Timestamp {
    Timestamp::try_from("2026-09-12T12:34:56.789Z".to_owned()).unwrap()
}
fn node_scope() -> ScopeRef {
    ScopeRef::Node {
        project_id: id(2),
        graph_id: id(3),
        node_id: id(4),
    }
}
fn library_scope() -> ScopeRef {
    ScopeRef::Library { library_id: id(1) }
}
fn env() -> BindingEnvironment {
    BindingEnvironment {
        owner: node_scope(),
        owning_library_id: id(1),
        owner_revision: rev(),
        run_id: Some(id(5)),
        asset_id: None,
        bindings: vec![],
        queries: vec![],
    }
}
fn int_type() -> Type {
    Type::builtin(Shape::Integer)
}
fn int(v: i64) -> Value {
    Value::Integer(Integer::new(v))
}
fn typed(v: i64) -> TypedValue {
    TypedValue {
        ty: int_type(),
        value: int(v),
    }
}
fn variable_dep(number: u64) -> Dependency {
    Dependency {
        coordinate: Coordinate::Variable {
            scope: library_scope(),
            variable_id: id(number),
        },
        projection: vec![],
    }
}
fn binding(number: u64, name: &str, ty: Type) -> SourceBinding {
    SourceBinding {
        spelling: format!("$library.{name}"),
        coordinate: variable_dep(number).coordinate,
        ty,
    }
}
fn expr(
    source: &str,
    mode: FieldMode,
    environment: BindingEnvironment,
    expected: &Type,
) -> Expression {
    Expression::compile(id(10_001), mode, source, environment, expected).unwrap()
}
fn fact(dep: Dependency, value: TypedValue) -> CapturedFact {
    CapturedFact {
        dependency: dep,
        ty: value.ty,
        revision: rev(),
        value_id: None,
        source: EffectiveSource::Captured,
        run_override: None,
        ast_digest: None,
        value: FactValue::Present(value.value),
        sensitivity: Sensitivity::Ordinary,
        portability: Portability::Portable,
        freshness: Freshness::Captured,
        required: true,
        dependencies: vec![],
    }
}
fn spec(roots: Vec<Dependency>, mut entries: Vec<CapturedFact>) -> SnapshotSpec {
    entries.sort_by_key(|f| f.dependency.key().unwrap());
    SnapshotSpec {
        snapshot_id: id(10_002),
        owning_library_id: id(1),
        project_id: id(2),
        captured_at: time(),
        versions: Versions::default(),
        audience: Audience {
            project_id: id(2),
            policy_digest: Digest::of_bytes(b"restricted"),
        },
        destination: CaptureDestination::LocalProject,
        consent: None,
        roots: sorted_dependencies(roots).unwrap(),
        entries,
        device_required: false,
        secret_requirements: vec![],
    }
}
fn evaluate(e: &Expression, facts: Vec<CapturedFact>) -> Result<EvaluatedValue> {
    let s = ContextSnapshot::build(spec(e.record().dependencies.clone(), facts))?;
    let c = FrozenContext::for_expression(&s, e, None)?;
    e.evaluate(&c)
}
fn error<T: Debug>(r: Result<T>, code: ErrorCode) {
    assert_eq!(r.unwrap_err().code, code);
}
#[test]
fn ordinary_strings_never_interpolate() {
    for source in [
        "$HOME",
        "${project.root}",
        "`$HOME`",
        "```bash\nwhoami\n```",
        "a\\q",
    ] {
        let field = CompiledField::compile(
            id(9),
            FieldMode::LiteralOnly,
            source,
            env(),
            &Type::string(),
        )
        .unwrap();
        let CompiledField::Literal(value) = field else {
            panic!()
        };
        assert_eq!(value.value, Value::String(source.into()));
    }
}

#[test]
fn template_escapes_and_utf8_spans_are_exact() {
    let e = expr(
        "é \\` \\\\ `\"ok\"`",
        FieldMode::Template,
        env(),
        &Type::string(),
    );
    assert_eq!(
        evaluate(&e, vec![]).unwrap().value,
        Value::String("é ` \\ ok".into())
    );
    let AstKind::Template { parts } = &e.record().ast.node else {
        panic!()
    };
    assert_eq!(parts[1].span, Span { start: 10, end: 14 });
    let display = Span { start: 3, end: 4 }.display("é\nx").unwrap();
    assert_eq!((display.start_line, display.start_column), (2, 1));
    assert!(Span { start: 1, end: 2 }.display("é").is_err());
}
#[test]
fn delimiters_and_fences_fail_closed() {
    for source in [
        "",
        "abc",
        "``",
        "` `",
        "`1",
        "`1` trailing",
        "`1``2`",
        "`\"x`y\"`",
    ] {
        assert!(
            Expression::compile(id(9), FieldMode::Expression, source, env(), &int_type()).is_err(),
            "{source}"
        );
    }
    for source in ["```photara-expr-v1\n1\n```", "```bash\n1\n```", "```"] {
        error(
            Expression::compile(id(9), FieldMode::Expression, source, env(), &int_type()),
            ErrorCode::UnsupportedVersion,
        );
    }
    for source in ["x\\q", "x\\", "a\nb"] {
        assert!(
            Expression::compile(id(9), FieldMode::Template, source, env(), &Type::string())
                .is_err()
        );
    }
}
#[test]
fn expression_strings_use_json_escapes_and_preserve_source() {
    let source = " \r\n`\"a\\u0060b\\n\\u00e9\"`\r\n";
    let e = expr(source, FieldMode::Expression, env(), &Type::string());
    assert_eq!(
        evaluate(&e, vec![]).unwrap().value,
        Value::String("a`b\né".into())
    );
    assert_eq!(e.record().source, source);
    assert_eq!(
        e.record().source_sha256,
        Digest::of_bytes(source.as_bytes())
    );
}
#[test]
fn casing_and_unknown_environment_or_functions_never_resolve() {
    for source in [
        "`$PATH`",
        "`$CUSTOM`",
        "`$home`",
        "`$PROJECT.root`",
        "`$Input.assets`",
        "`$delivery`",
        "`now()`",
        "`system(\"x\")`",
        "`Path.Join()`",
        "`$project.assets`",
    ] {
        assert!(
            Expression::compile(id(9), FieldMode::Expression, source, env(), &int_type()).is_err(),
            "{source}"
        );
    }
}
#[test]
fn ast_semantics_exclude_formatting_and_expression_identity() {
    let a = expr("`1 + 2 * 3`", FieldMode::Expression, env(), &int_type());
    let b = Expression::compile(
        id(99),
        FieldMode::Expression,
        " \n`1+2*3` ",
        env(),
        &int_type(),
    )
    .unwrap();
    assert_eq!(a.record().ast_digest, b.record().ast_digest);
    assert_ne!(a.record().source_sha256, b.record().source_sha256);
    assert_eq!(evaluate(&a, vec![]).unwrap().value, int(7));
}
#[test]
fn source_ast_correspondence_versions_and_unknown_fields_are_checked() {
    let e = expr("`1+2`", FieldMode::Expression, env(), &int_type());
    Expression::verify(e.record().clone()).unwrap();
    let mut r = e.record().clone();
    r.source = "`1+3`".into();
    error(Expression::verify(r), ErrorCode::SourceAstMismatch);
    let mut r = e.record().clone();
    r.ast.ty = Type::string();
    assert!(Expression::verify(r).is_err());
    let mut r = e.record().clone();
    r.compiler_version = Version::new(2).unwrap();
    error(Expression::verify(r), ErrorCode::UnsupportedVersion);
    let mut v = serde_json::to_value(e.record()).unwrap();
    v["extra"] = json!(true);
    assert!(Expression::from_json(&serde_json::to_vec(&v).unwrap()).is_err());
    let wire = serde_json::to_vec(e.record()).unwrap();
    assert_eq!(Expression::from_json(&wire).unwrap(), e);
}
#[test]
fn checked_i64_arithmetic_and_boolean_precedence() {
    for (source, expected) in [
        ("`-9223372036854775808`", i64::MIN),
        ("`9223372036854775807`", i64::MAX),
        ("`(8-3)*2`", 10),
    ] {
        assert_eq!(
            evaluate(
                &expr(source, FieldMode::Expression, env(), &int_type()),
                vec![]
            )
            .unwrap()
            .value,
            int(expected)
        );
    }
    for source in [
        "`9223372036854775807+1`",
        "`-9223372036854775808-1`",
        "`9223372036854775807*2`",
    ] {
        error(
            evaluate(
                &expr(source, FieldMode::Expression, env(), &int_type()),
                vec![],
            ),
            ErrorCode::Overflow,
        );
    }
    for source in ["`9223372036854775808`", "`01`", "`-0`", "`1.2`", "`1/2`"] {
        assert!(
            Expression::compile(id(9), FieldMode::Expression, source, env(), &int_type()).is_err()
        );
    }
    let e = expr(
        "`true or false and not true`",
        FieldMode::Expression,
        env(),
        &Type::builtin(Shape::Bool),
    );
    assert_eq!(evaluate(&e, vec![]).unwrap().value, Value::Bool(true));
}
#[test]
fn no_implicit_scalar_coercion_or_resource_stringification() {
    for source in ["`1+true`", "`\"1\"+\"2\"`", "`if(true,1,\"2\")`"] {
        assert!(
            Expression::compile(id(9), FieldMode::Expression, source, env(), &int_type()).is_err()
        );
    }
    assert!(
        Expression::compile(
            id(9),
            FieldMode::Template,
            "count `1`",
            env(),
            &Type::string()
        )
        .is_err()
    );
    let e = expr(
        "count `text.format(7,\"und\",1)`",
        FieldMode::Template,
        env(),
        &Type::string(),
    );
    assert_eq!(
        evaluate(&e, vec![]).unwrap().value,
        Value::String("count 7".into())
    );
    let e = expr(
        "`text.format(1,\"system\",1)`",
        FieldMode::Expression,
        env(),
        &Type::string(),
    );
    error(evaluate(&e, vec![]), ErrorCode::UnsupportedVersion);
}
#[test]
fn both_branches_typecheck_but_only_selected_branch_executes() {
    let e = expr(
        "`if(false,9223372036854775807+1,2)`",
        FieldMode::Expression,
        env(),
        &int_type(),
    );
    assert_eq!(evaluate(&e, vec![]).unwrap().value, int(2));
    let mut env = env();
    env.bindings = vec![binding(20, "a", int_type()), binding(21, "b", int_type())];
    let e = expr(
        "`if(true,$library.a,$library.b)`",
        FieldMode::Expression,
        env,
        &int_type(),
    );
    assert_eq!(e.record().dependencies.len(), 2);
    assert_eq!(
        evaluate(
            &e,
            vec![
                fact(variable_dep(20), typed(1)),
                fact(variable_dep(21), typed(2))
            ]
        )
        .unwrap()
        .value,
        int(1)
    );
}
#[test]
fn literal_lists_records_and_registered_field_projections() {
    let ty = Type::builtin(Shape::Record {
        fields: [
            (n("count"), int_type()),
            (
                n("labels"),
                Type::builtin(Shape::List {
                    element: Box::new(Type::string()),
                    max_items: 2,
                }),
            ),
        ]
        .into(),
    });
    let e = expr(
        "`{\"labels\":[\"a\",\"b\"],\"count\":2}`",
        FieldMode::Expression,
        env(),
        &ty,
    );
    evaluate(&e, vec![]).unwrap();
    assert!(
        Expression::compile(
            id(9),
            FieldMode::Expression,
            "`{\"count\":2,\"count\":3}`",
            env(),
            &ty
        )
        .is_err()
    );
    let mut environment = env();
    environment.bindings.push(binding(20, "settings", ty));
    let e = expr(
        "`$library.settings.count`",
        FieldMode::Expression,
        environment,
        &int_type(),
    );
    assert_eq!(e.record().dependencies[0].projection, vec![n("count")]);
}
#[test]
fn decimal_rational_timestamp_and_optional_wire_boundaries() {
    for (coefficient, scale) in [
        ("0", 0),
        ("-120", 2),
        ("99999999999999999999999999999999999999", 18),
    ] {
        Decimal {
            coefficient: coefficient.into(),
            scale,
        }
        .validate()
        .unwrap();
    }
    for (coefficient, scale) in [
        ("+1", 0),
        ("01", 0),
        ("-0", 0),
        ("1", 19),
        ("1.0", 1),
        ("100000000000000000000000000000000000000", 0),
    ] {
        assert!(
            Decimal {
                coefficient: coefficient.into(),
                scale
            }
            .validate()
            .is_err()
        );
    }
    for s in ["2000-02-29T00:00:00.000Z", "9999-12-31T23:59:59.999Z"] {
        Timestamp::try_from(s.to_owned()).unwrap();
    }
    for s in [
        "1900-02-29T00:00:00.000Z",
        "0000-01-01T00:00:00.000Z",
        "2026-01-01T24:00:00.000Z",
        "2026-01-01T00:00:60.000Z",
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:00.000+00:00",
    ] {
        assert!(Timestamp::try_from(s.to_owned()).is_err());
    }
    let ty = Type::builtin(Shape::Rational {
        unit: q("photara.seconds"),
    });
    assert!(
        ty.check(&Value::Rational {
            numerator: Integer::new(1),
            denominator: Integer::new(0),
            unit: q("photara.seconds")
        })
        .is_err()
    );
    let optional = Type::builtin(Shape::Optional {
        element: Box::new(int_type()),
    });
    optional.check(&Value::Optional(None)).unwrap();
    assert!(optional.check(&Value::Null).is_err());
}
#[test]
fn optional_fallback_means_absence_only() {
    let optional = Type::builtin(Shape::Optional {
        element: Box::new(int_type()),
    });
    let mut env = env();
    env.bindings.push(binding(20, "choice", optional.clone()));
    let e = expr(
        "`optional.or_else($library.choice,4)`",
        FieldMode::Expression,
        env,
        &int_type(),
    );
    let mut f = fact(
        variable_dep(20),
        TypedValue {
            ty: optional,
            value: Value::Optional(None),
        },
    );
    f.value = FactValue::Absent;
    f.required = false;
    assert_eq!(evaluate(&e, vec![f.clone()]).unwrap().value, int(4));
    for code in [
        ErrorCode::Forbidden,
        ErrorCode::Revoked,
        ErrorCode::Unavailable,
        ErrorCode::TypeMismatch,
    ] {
        f.value = FactValue::Unavailable(code);
        error(evaluate(&e, vec![f.clone()]), code);
    }
}
#[test]
fn parser_and_ast_limits_do_not_truncate() {
    error(
        CompiledField::compile(
            id(9),
            FieldMode::LiteralOnly,
            &"x".repeat(SOURCE_LIMIT + 1),
            env(),
            &Type::string(),
        ),
        ErrorCode::LimitExceeded,
    );
    let field = CompiledField::compile(
        id(9),
        FieldMode::LiteralOnly,
        &"x".repeat(SOURCE_LIMIT),
        env(),
        &Type::string(),
    )
    .unwrap();
    assert!(matches!(field, CompiledField::Literal(_)));
    let deep = format!("`{}1{}`", "(".repeat(33), ")".repeat(33));
    error(
        Expression::compile(id(9), FieldMode::Expression, &deep, env(), &int_type()),
        ErrorCode::LimitExceeded,
    );
    let nodes = format!("`{}`", vec!["1"; 1025].join("+"));
    error(
        Expression::compile(id(9), FieldMode::Expression, &nodes, env(), &int_type()),
        ErrorCode::LimitExceeded,
    );
}
#[test]
fn direct_dependencies_include_all_branches_and_enforce_bound() {
    let mut environment = env();
    for i in 0..257 {
        environment
            .bindings
            .push(binding(100 + i, &format!("v{i}"), int_type()));
    }
    let source = format!(
        "`[{}]`",
        (0..257)
            .map(|i| format!("$library.v{i}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let ty = Type::builtin(Shape::List {
        element: Box::new(int_type()),
        max_items: 1000,
    });
    error(
        Expression::compile(id(9), FieldMode::Expression, &source, environment, &ty),
        ErrorCode::LimitExceeded,
    );
}
#[test]
fn directional_scope_matrix_prevents_children_siblings_and_cross_library() {
    let l = library_scope();
    let p = ScopeRef::Project { project_id: id(2) };
    let g = ScopeRef::Graph {
        project_id: id(2),
        graph_id: id(3),
    };
    let node = node_scope();
    for (i, owner) in [l, p, g, node].into_iter().enumerate() {
        for (j, target) in [l, p, g, node].into_iter().enumerate() {
            assert_eq!(scope_allows(owner, target, id(1)), j <= i);
        }
    }
    assert!(!scope_allows(
        node,
        ScopeRef::Library { library_id: id(99) },
        id(1)
    ));
    assert!(!scope_allows(
        node,
        ScopeRef::Graph {
            project_id: id(2),
            graph_id: id(99)
        },
        id(1)
    ));
    let mut e = env();
    e.owner = l;
    e.bindings.push(SourceBinding {
        spelling: "$project.value".into(),
        coordinate: Coordinate::Variable {
            scope: p,
            variable_id: id(10),
        },
        ty: int_type(),
    });
    error(e.validate(), ErrorCode::ScopeMismatch);
}
#[test]
fn input_asset_and_host_bindings_are_explicit_and_typed() {
    let mut environment = env();
    environment.bindings.push(SourceBinding {
        spelling: "$HOME".into(),
        coordinate: Coordinate::HostPlace {
            place: HostPlace::Home,
        },
        ty: Type::builtin(Shape::Resource),
    });
    let e = expr(
        "`path.join($HOME,\"exports\",\"a\")`",
        FieldMode::Expression,
        environment,
        &Type::builtin(Shape::Resource),
    );
    let mut s = spec(vec![], vec![]);
    s.device_required = true;
    let snapshot = ContextSnapshot::build(s).unwrap();
    let d = DeviceContextSnapshot::build(
        id(11),
        id(5),
        vec![DeviceFact {
            place: HostPlace::Home,
            binding_id: id(12),
            generation: 1,
            availability: DeviceAvailability::Ready,
            temp_lease: None,
        }],
    )
    .unwrap();
    let c = FrozenContext::for_expression(&snapshot, &e, Some(d)).unwrap();
    let Value::Resource(resource) = e.evaluate(&c).unwrap().value else {
        panic!()
    };
    assert_eq!(resource.components, vec!["exports", "a"]);
    let mut environment = env();
    environment.bindings.push(SourceBinding {
        spelling: "$asset.metadata.exif.camera.model".into(),
        coordinate: Coordinate::AssetMetadata {
            project_id: id(2),
            graph_id: id(3),
            node_id: id(4),
            asset_id: id(30),
            representation_id: id(31),
            content_revision_id: id(32),
            field: q("exif.camera.model"),
        },
        ty: Type::string(),
    });
    error(environment.validate(), ErrorCode::AssetContextRequired);
}
#[test]
fn path_join_remains_a_descriptor_without_package_write_or_traversal() {
    let root = ResourceValue {
        descriptor: ResourceDescriptor::ProjectRoot { project_id: id(2) },
        components: vec![],
    };
    for part in [
        "..", "/tmp", "C:\\x", "objects", "HEAD", "locks", "a/b", "con", "x:",
    ] {
        assert!(root.join(&[part.into()]).is_err(), "{part}");
    }
    root.join(&["exports".into(), "café.tif".into()]).unwrap();
}
fn variable(number: u64, name: &str) -> VariableAggregate {
    VariableAggregate::try_from(VariableSpec {
        variable_id: id(number),
        scope: library_scope(),
        owning_library_id: id(1),
        definition_version: Version::FIRST,
        namespace: q("example.variables"),
        name: n(name),
        claimed_names: vec![n(name)],
        label: name.into(),
        description: String::new(),
        ty: int_type(),
        default: Some(Binding::Literal(Box::new(typed(1)))),
        current: Some(Binding::Literal(Box::new(typed(2)))),
        value_id: Some(id(number + 1000)),
        revision: LocalRevision::INITIAL,
        owner_revision: rev(),
        state: VariableState::Active,
        allow_run_override: true,
        sensitivity: Sensitivity::Ordinary,
        portability: Portability::Portable,
        origin: ValueOrigin::Manual,
        created_at: time(),
        updated_at: time(),
    })
    .unwrap()
}
#[test]
fn effective_precedence_clear_and_override_do_not_mutate_defaults() {
    let v = variable(20, "count");
    let o = RunOverride {
        override_id: id(40),
        run_id: id(5),
        target: v.dependency(),
        expected_revision: LocalRevision::INITIAL,
        value: typed(3),
        sensitivity: Sensitivity::Ordinary,
        portability: Portability::Portable,
    };
    let resolved = resolve_variables(
        std::slice::from_ref(&v),
        &[],
        std::slice::from_ref(&o),
        id(5),
    )
    .unwrap();
    assert_eq!(resolved[0].fact.resolved().unwrap().value, int(3));
    assert_eq!(resolved[0].fact.source, EffectiveSource::RunOverride);
    assert_eq!(
        resolve_variables(std::slice::from_ref(&v), &[], &[], id(5)).unwrap()[0]
            .fact
            .resolved()
            .unwrap()
            .value,
        int(2)
    );
    let cleared = v
        .plan_clear(
            LocalRevision::INITIAL,
            RevisionCoordinate::Local {
                revision: LocalRevision::new(2).unwrap(),
            },
            time(),
        )
        .unwrap();
    assert_eq!(cleared.spec().value_id, v.spec().value_id);
    assert_eq!(
        resolve_variables(&[cleared], &[], &[], id(5)).unwrap()[0]
            .fact
            .resolved()
            .unwrap()
            .value,
        int(1)
    );
    let mut bad = o;
    bad.expected_revision = LocalRevision::new(2).unwrap();
    error(
        resolve_variables(&[v], &[], &[bad], id(5)),
        ErrorCode::Conflict,
    );
}
#[test]
fn rename_retains_id_binding_and_tombstone_never_retargets() {
    let v = variable(20, "before");
    let mut environment = env();
    environment.owner = library_scope();
    environment.bindings.push(binding(20, "before", int_type()));
    let expression = expr(
        "`$library.before`",
        FieldMode::Expression,
        environment,
        &int_type(),
    );
    let mut s = v.spec().clone();
    s.name = n("after");
    s.claimed_names = vec![n("after"), n("before")];
    let renamed = VariableAggregate::try_from(s).unwrap();
    assert_eq!(expression.record().dependencies[0], renamed.dependency());
    let mut consumer = variable(21, "consumer").spec().clone();
    consumer.current = Some(Binding::Expression(Box::new(expression.record().clone())));
    consumer.default = None;
    let results = resolve_variables(
        &[
            renamed.clone(),
            VariableAggregate::try_from(consumer.clone()).unwrap(),
        ],
        &[],
        &[],
        id(5),
    )
    .unwrap();
    assert_eq!(results[1].fact.resolved().unwrap().value, int(2));
    let mut tombstone = renamed.spec().clone();
    tombstone.state = VariableState::Tombstoned;
    error(
        resolve_variables(
            &[
                VariableAggregate::try_from(tombstone).unwrap(),
                VariableAggregate::try_from(consumer).unwrap(),
            ],
            &[],
            &[],
            id(5),
        ),
        ErrorCode::Tombstoned,
    );
}
#[test]
fn variable_name_claims_defaults_and_overrides_cannot_hide_cycles() {
    let mut a = variable(20, "a").spec().clone();
    let mut b = variable(21, "b").spec().clone();
    let mut environment = env();
    environment.owner = library_scope();
    environment.bindings = vec![binding(20, "a", int_type()), binding(21, "b", int_type())];
    a.default = Some(Binding::Expression(Box::new(
        expr(
            "`if(false,$library.b,1)`",
            FieldMode::Expression,
            environment.clone(),
            &int_type(),
        )
        .record()
        .clone(),
    )));
    b.default = Some(Binding::Expression(Box::new(
        expr(
            "`$library.a`",
            FieldMode::Expression,
            environment,
            &int_type(),
        )
        .record()
        .clone(),
    )));
    error(
        variable_order(&[
            VariableAggregate::try_from(a).unwrap(),
            VariableAggregate::try_from(b).unwrap(),
        ]),
        ErrorCode::Cyclic,
    );
    error(
        variable_order(&[variable(20, "same"), variable(21, "same")]),
        ErrorCode::Conflict,
    );
}
#[test]
fn variables_recursively_forbid_workflow_families_and_type_changes() {
    for shape in [
        Shape::AssetSet,
        Shape::Metadata {
            element: Box::new(int_type()),
        },
        Shape::List {
            element: Box::new(Type::builtin(Shape::AssetSet)),
            max_items: 1,
        },
    ] {
        let ty = Type::builtin(shape);
        error(ty.validate(true), ErrorCode::Privacy);
    }
    let mut s = variable(20, "a").spec().clone();
    s.ty = Type::string();
    error(VariableAggregate::try_from(s), ErrorCode::TypeMismatch);
}

#[test]
fn snapshot_hashes_exclude_provenance_but_include_requested_values() {
    let f = fact(variable_dep(20), typed(1));
    let a = ContextSnapshot::build(spec(vec![f.dependency.clone()], vec![f])).unwrap();
    let mut s = a.spec().clone();
    s.snapshot_id = id(200);
    s.captured_at = Timestamp::try_from("2026-09-13T00:00:00.000Z".to_owned()).unwrap();
    let b = ContextSnapshot::build(s).unwrap();
    assert_eq!(a.content_digest(), b.content_digest());
    let mut s = a.spec().clone();
    s.entries[0].value = FactValue::Present(int(2));
    let b = ContextSnapshot::build(s).unwrap();
    assert_ne!(a.content_digest(), b.content_digest());
    let bytes = photara_core::canonical_json(&a).unwrap();
    assert_eq!(ContextSnapshot::from_json(&bytes).unwrap(), a);
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["content_digest"] = serde_json::to_value(Digest::of_bytes(b"bad")).unwrap();
    assert!(ContextSnapshot::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
}
#[test]
fn snapshots_reject_incomplete_excess_duplicate_and_cyclic_closure() {
    let first = fact(variable_dep(20), typed(1));
    let dependency = variable_dep(21);
    let mut snapshot = spec(vec![first.dependency.clone()], vec![first.clone()]);
    snapshot.entries[0].dependencies = vec![dependency.clone()];
    error(ContextSnapshot::build(snapshot), ErrorCode::Incomplete);
    let snapshot = spec(
        vec![first.dependency.clone()],
        vec![first.clone(), fact(dependency.clone(), typed(2))],
    );
    error(ContextSnapshot::build(snapshot), ErrorCode::Incomplete);
    let snapshot = spec(
        vec![first.dependency.clone()],
        vec![first.clone(), first.clone()],
    );
    error(ContextSnapshot::build(snapshot), ErrorCode::Conflict);
    let mut cycle_first = first;
    let mut cycle_second = fact(dependency.clone(), typed(2));
    cycle_first.dependencies = vec![dependency];
    cycle_second.dependencies = vec![cycle_first.dependency.clone()];
    let snapshot = spec(
        vec![cycle_first.dependency.clone()],
        vec![cycle_first, cycle_second],
    );
    error(ContextSnapshot::build(snapshot), ErrorCode::Cyclic);
}
#[test]
fn snapshots_capture_negative_facts_and_staleness_without_faking_replay() {
    let mut f = fact(variable_dep(20), typed(1));
    let mut digests = std::collections::BTreeSet::new();
    for code in [
        ErrorCode::Unknown,
        ErrorCode::Forbidden,
        ErrorCode::Revoked,
        ErrorCode::Tombstoned,
        ErrorCode::Unavailable,
        ErrorCode::StaleFingerprint,
        ErrorCode::Ambiguous,
        ErrorCode::Conflict,
    ] {
        f.value = FactValue::Unavailable(code);
        let s = ContextSnapshot::build(spec(vec![f.dependency.clone()], vec![f.clone()])).unwrap();
        assert_eq!(s.completeness(), Completeness::Incomplete);
        assert_eq!(s.replayability(), Replayability::Blocked);
        assert!(digests.insert(s.content_digest().to_string()));
    }
    f.value = FactValue::Present(int(1));
    f.freshness = Freshness::LivePreview;
    error(
        ContextSnapshot::build(spec(vec![f.dependency.clone()], vec![f.clone()])),
        ErrorCode::Stale,
    );
    f.freshness = Freshness::Stale;
    let s = ContextSnapshot::build(spec(vec![f.dependency.clone()], vec![f])).unwrap();
    assert_eq!(s.replayability(), Replayability::Blocked);
}
#[test]
fn capture_consent_pins_projection_audience_and_sensitivity() {
    let mut f = fact(variable_dep(20), typed(1));
    f.sensitivity = Sensitivity::Restricted;
    f.portability = Portability::CaptureConsentRequired;
    let mut s = spec(vec![f.dependency.clone()], vec![f]);
    error(
        ContextSnapshot::build(s.clone()),
        ErrorCode::ConsentRequired,
    );
    s.consent = Some(CaptureConsent {
        audience: s.audience.clone(),
        projection_digest: projection_digest(&s.entries).unwrap(),
        sensitivity_ceiling: Sensitivity::Restricted,
        granted: true,
    });
    let captured = ContextSnapshot::build(s.clone()).unwrap();
    let mut audience = s.audience.clone();
    audience.policy_digest = Digest::of_bytes(b"widened");
    error(
        captured.validate_audience(&audience),
        ErrorCode::ConsentRequired,
    );
    s.entries[0].value = FactValue::Present(int(9));
    error(
        ContextSnapshot::build(s.clone()),
        ErrorCode::ConsentRequired,
    );
    s.entries[0].value = FactValue::Present(int(1));
    s.destination = CaptureDestination::SharedProject;
    error(ContextSnapshot::build(s), ErrorCode::Privacy);
}
#[test]
fn derived_facts_cannot_declassify_or_capture_host_only_payloads() {
    let mut a = fact(variable_dep(20), typed(1));
    a.sensitivity = Sensitivity::Personal;
    let mut b = fact(variable_dep(21), typed(2));
    b.dependencies = vec![a.dependency.clone()];
    error(
        ContextSnapshot::build(spec(vec![b.dependency.clone()], vec![a, b])),
        ErrorCode::Privacy,
    );
    let mut f = fact(
        variable_dep(20),
        TypedValue {
            ty: Type::builtin(Shape::Resource),
            value: Value::Resource(Box::new(ResourceValue {
                descriptor: ResourceDescriptor::HostPlace {
                    place: HostPlace::Home,
                },
                components: vec![],
            })),
        },
    );
    error(
        ContextSnapshot::build(spec(vec![f.dependency.clone()], vec![f.clone()])),
        ErrorCode::Privacy,
    );
    f.value = FactValue::Unavailable(ErrorCode::Unavailable);
    f.portability = Portability::HostOnly;
    error(
        ContextSnapshot::build(spec(vec![f.dependency.clone()], vec![f])),
        ErrorCode::Privacy,
    );
}
#[test]
fn snapshot_project_ownership_and_metadata_input_scope_are_explicit() {
    let f = fact(
        variable_dep(20),
        TypedValue {
            ty: Type::builtin(Shape::Resource),
            value: Value::Resource(Box::new(ResourceValue {
                descriptor: ResourceDescriptor::ProjectRoot {
                    project_id: id(999),
                },
                components: vec![],
            })),
        },
    );
    error(
        ContextSnapshot::build(spec(vec![f.dependency.clone()], vec![f])),
        ErrorCode::ScopeMismatch,
    );
    let mut s = spec(vec![], vec![]);
    s.audience.project_id = id(999);
    error(ContextSnapshot::build(s), ErrorCode::ScopeMismatch);
}
#[test]
fn declared_subset_stays_frozen_and_does_not_include_unrelated_values() {
    let mut environment = env();
    environment.bindings.push(binding(20, "a", int_type()));
    let e = expr(
        "`$library.a`",
        FieldMode::Expression,
        environment,
        &int_type(),
    );
    let a = fact(variable_dep(20), typed(1));
    let b = fact(variable_dep(21), typed(2));
    let snapshot = ContextSnapshot::build(spec(
        vec![a.dependency.clone(), b.dependency.clone()],
        vec![a, b],
    ))
    .unwrap();
    let frozen = FrozenContext::for_expression(&snapshot, &e, None).unwrap();
    let mut s = snapshot.spec().clone();
    s.entries[0].value = FactValue::Present(int(9));
    let refreshed = ContextSnapshot::build(s).unwrap();
    assert_eq!(e.evaluate(&frozen).unwrap().value, int(1));
    assert_eq!(
        e.evaluate(&FrozenContext::for_expression(&refreshed, &e, None).unwrap())
            .unwrap()
            .value,
        int(9)
    );
}
#[test]
fn device_context_digest_changes_with_binding_generation_availability_and_temp_lease() {
    let f = DeviceFact {
        place: HostPlace::Home,
        binding_id: id(50),
        generation: 1,
        availability: DeviceAvailability::Ready,
        temp_lease: None,
    };
    let a = DeviceContextSnapshot::build(id(11), id(5), vec![f.clone()]).unwrap();
    let mut next = f.clone();
    next.generation = 2;
    assert_ne!(
        a.digest(),
        DeviceContextSnapshot::build(id(11), id(5), vec![next])
            .unwrap()
            .digest()
    );
    let mut next = f;
    next.availability = DeviceAvailability::Denied;
    assert_ne!(
        a.digest(),
        DeviceContextSnapshot::build(id(11), id(5), vec![next])
            .unwrap()
            .digest()
    );
    let f = DeviceFact {
        place: HostPlace::Temp,
        binding_id: id(51),
        generation: 1,
        availability: DeviceAvailability::Ready,
        temp_lease: Some(id(52)),
    };
    let a = DeviceContextSnapshot::build(id(11), id(5), vec![f.clone()]).unwrap();
    let mut next = f;
    next.temp_lease = Some(id(53));
    assert_ne!(
        a.digest(),
        DeviceContextSnapshot::build(id(11), id(6), vec![next])
            .unwrap()
            .digest()
    );
    assert_eq!(format!("{a:?}"), "DeviceContextSnapshot(<private>)");
}
fn boundary() -> AuthorizationBoundary {
    AuthorizationBoundary {
        project_id: id(2),
        principal: Principal::Account { account_id: id(60) },
        generation: AuthorizationGeneration::new(1).unwrap(),
        audience: Audience {
            project_id: id(2),
            policy_digest: Digest::of_bytes(b"restricted"),
        },
        sensitivity_ceiling: Sensitivity::Personal,
    }
}
fn key_spec(dependencies: Vec<Dependency>) -> NodeKeySpec {
    NodeKeySpec {
        key_version: Version::new(2).unwrap(),
        contract_version: Version::FIRST,
        versions: Versions::default(),
        node_id: id(4),
        graph_id: id(3),
        definition: DefinitionPin {
            package_id: q("example.node"),
            package_version: photara_core::PackageVersion::new(1, 0, 0),
            definition_id: q("example.node.test"),
            definition_version: Version::FIRST,
            implementation_digest: Digest::of_bytes(b"implementation"),
        },
        configuration: typed(0),
        authored_state: None,
        inputs: vec![],
        dependencies,
        environment_digest: Digest::of_bytes(b"environment"),
        authorization: boundary(),
        eligibility: CacheEligibility::Pure,
        location: CacheLocation::Project,
    }
}
#[test]
fn cache_v2_only_dirties_declared_context_closure() {
    let a = fact(variable_dep(20), typed(1));
    let b = fact(variable_dep(21), typed(2));
    let snapshot = ContextSnapshot::build(spec(
        vec![a.dependency.clone(), b.dependency.clone()],
        vec![a, b],
    ))
    .unwrap();
    let k = key_spec(vec![variable_dep(20)]);
    let before = NodeCacheKey::build(&k, &snapshot, None).unwrap();
    let mut s = snapshot.spec().clone();
    s.entries[1].value = FactValue::Present(int(9));
    let changed = ContextSnapshot::build(s).unwrap();
    assert_eq!(
        before.digest(),
        NodeCacheKey::build(&k, &changed, None).unwrap().digest()
    );
    let mut s = snapshot.spec().clone();
    s.entries[0].value = FactValue::Present(int(9));
    let changed = ContextSnapshot::build(s).unwrap();
    assert_ne!(
        before.digest(),
        NodeCacheKey::build(&k, &changed, None).unwrap().digest()
    );
    let mut k2 = k.clone();
    k2.definition.implementation_digest = Digest::of_bytes(b"new");
    assert_ne!(
        before.digest(),
        NodeCacheKey::build(&k2, &snapshot, None).unwrap().digest()
    );
}
#[test]
fn cache_lookup_requires_current_project_principal_generation_and_device() {
    let snapshot = ContextSnapshot::build(spec(vec![], vec![])).unwrap();
    let k = NodeCacheKey::build(&key_spec(vec![]), &snapshot, None).unwrap();
    k.authorize_lookup(&boundary(), true, None).unwrap();
    error(
        k.authorize_lookup(&boundary(), false, None),
        ErrorCode::Revoked,
    );
    let mut b = boundary();
    b.generation = AuthorizationGeneration::new(2).unwrap();
    error(k.authorize_lookup(&b, true, None), ErrorCode::Forbidden);
    let mut b = boundary();
    b.principal = Principal::Account { account_id: id(61) };
    error(k.authorize_lookup(&b, true, None), ErrorCode::Forbidden);
    let mut b = boundary();
    b.project_id = id(62);
    error(k.authorize_lookup(&b, true, None), ErrorCode::Forbidden);
    error(
        k.authorize_lookup(&boundary(), true, Some(Digest::of_bytes(b"device"))),
        ErrorCode::Stale,
    );
}
#[test]
fn cache_rejects_effects_secrets_nondeterminism_and_shared_device_results() {
    let snapshot = ContextSnapshot::build(spec(vec![], vec![])).unwrap();
    for eligibility in [
        CacheEligibility::Effect,
        CacheEligibility::SecretTainted,
        CacheEligibility::NonDeterministic,
    ] {
        let mut k = key_spec(vec![]);
        k.eligibility = eligibility;
        error(
            NodeCacheKey::build(&k, &snapshot, None),
            ErrorCode::Forbidden,
        );
    }
    let d = DeviceContextSnapshot::build(id(11), id(5), vec![]).unwrap();
    error(
        NodeCacheKey::build(&key_spec(vec![]), &snapshot, Some(&d)),
        ErrorCode::Privacy,
    );
    let mut s = spec(vec![], vec![]);
    s.secret_requirements = vec![n("credential")];
    assert_eq!(
        ContextSnapshot::build(s).unwrap().replayability(),
        Replayability::Blocked
    );
}
#[test]
fn graph_cache_key_includes_context_even_without_nodes() {
    let f = fact(variable_dep(20), typed(1));
    let a = ContextSnapshot::build(spec(vec![f.dependency.clone()], vec![f])).unwrap();
    let mut s = a.spec().clone();
    s.entries[0].value = FactValue::Present(int(2));
    let b = ContextSnapshot::build(s).unwrap();
    let environment = Digest::of_bytes(b"env");
    assert_ne!(
        graph_cache_key(id(3), &[], &a, environment, None, &boundary()).unwrap(),
        graph_cache_key(id(3), &[], &b, environment, None, &boundary()).unwrap()
    );
}
fn object(label: &[u8]) -> ObjectRef {
    ObjectRef {
        kind: ObjectKind::Json,
        sha256: Digest::of_bytes(label),
        byte_length: DecimalU64::new(label.len() as u64),
    }
}
fn proposal() -> VariableChangeProposal {
    VariableChangeProposal::try_from(ProposalSpec {
        proposal_id: ProposalId::try_from("62000000-0000-4000-8000-000000010003".to_owned())
            .unwrap(),
        operation_id: id(71),
        target: variable_dep(20),
        expected_aggregate_revision: LocalRevision::INITIAL,
        expected_owner_revision: rev(),
        literal: typed(9),
        sensitivity: Sensitivity::Ordinary,
        portability: Portability::Portable,
        project_id: id(2),
        run_id: id(5),
        attempt_id: id(72),
        snapshot_id: id(73),
        snapshot: object(b"snapshot"),
        snapshot_digest: Digest::of_bytes(b"context"),
        output_digest: Digest::of_bytes(b"output"),
    })
    .unwrap()
}
fn apply_evidence(p: &VariableChangeProposal) -> ApplyEvidence {
    let s = p.spec();
    ApplyEvidence {
        run_id: s.run_id,
        outcome: RunOutcome::Succeeded,
        explicit_acceptance: true,
        targets: vec![TargetEvidence {
            target: s.target.clone(),
            current_aggregate_revision: s.expected_aggregate_revision,
            current_owner_revision: s.expected_owner_revision.clone(),
            expected_type: s.literal.ty.clone(),
            sensitivity: Sensitivity::Ordinary,
            portability: Portability::Portable,
            declared: true,
            currently_authorized: true,
        }],
        sources: vec![ProposalSourceEvidence {
            run_id: s.run_id,
            attempt_id: s.attempt_id,
            snapshot_id: s.snapshot_id,
            snapshot: s.snapshot.clone(),
            snapshot_digest: s.snapshot_digest,
            output_digest: s.output_digest,
            sensitivity: Sensitivity::Ordinary,
            portability: Portability::Portable,
        }],
    }
}
#[test]
fn proposals_plan_only_after_success_explicit_acceptance_and_exact_source() {
    let p = proposal();
    let evidence = apply_evidence(&p);
    let plan = ApplyPlan::prepare(vec![p.clone()], &evidence).unwrap();
    assert_eq!(plan.authority(), Authority::Library { library_id: id(1) });
    for outcome in [
        RunOutcome::Failed,
        RunOutcome::Cancelled,
        RunOutcome::Interrupted,
        RunOutcome::Running,
    ] {
        let mut e = evidence.clone();
        e.outcome = outcome;
        error(
            ApplyPlan::prepare(vec![p.clone()], &e),
            ErrorCode::Forbidden,
        );
    }
    let mut e = evidence.clone();
    e.explicit_acceptance = false;
    error(
        ApplyPlan::prepare(vec![p.clone()], &e),
        ErrorCode::Forbidden,
    );
    let mut e = evidence;
    e.sources[0].snapshot_digest = Digest::of_bytes(b"other");
    error(
        ApplyPlan::prepare(vec![p], &e),
        ErrorCode::SourceAstMismatch,
    );
}
#[test]
fn proposal_planning_rechecks_cas_declaration_privacy_and_single_authority() {
    let p = proposal();
    let mut e = apply_evidence(&p);
    e.targets[0].current_aggregate_revision = LocalRevision::new(2).unwrap();
    error(ApplyPlan::prepare(vec![p.clone()], &e), ErrorCode::Conflict);
    let mut e = apply_evidence(&p);
    e.targets[0].currently_authorized = false;
    error(
        ApplyPlan::prepare(vec![p.clone()], &e),
        ErrorCode::Forbidden,
    );
    let mut e = apply_evidence(&p);
    e.sources[0].sensitivity = Sensitivity::Restricted;
    error(ApplyPlan::prepare(vec![p.clone()], &e), ErrorCode::Privacy);
    error(
        ApplyPlan::prepare(vec![p.clone(), p.clone()], &apply_evidence(&p)),
        ErrorCode::Conflict,
    );
    let mut s = p.spec().clone();
    s.operation_id = id(99);
    s.target = Dependency {
        coordinate: Coordinate::Variable {
            scope: ScopeRef::Project { project_id: id(2) },
            variable_id: id(88),
        },
        projection: vec![],
    };
    let second = VariableChangeProposal::try_from(s).unwrap();
    let mut e = apply_evidence(&p);
    e.targets.extend(apply_evidence(&second).targets);
    error(
        ApplyPlan::prepare(vec![p, second], &e),
        ErrorCode::ScopeMismatch,
    );
}
fn receipt(id_number: u64, outcome: ReceiptOutcome, prior: Vec<ReceiptId>) -> ApplyReceipt {
    let p = proposal();
    ApplyReceipt::try_from(ReceiptSpec {
        receipt_id: id(id_number),
        operation_id: p.spec().operation_id,
        request_digest: p.request_digest().unwrap(),
        authority: Authority::Library { library_id: id(1) },
        outcome,
        resulting_revisions: if outcome == ReceiptOutcome::Applied {
            vec![rev()]
        } else {
            vec![]
        },
        prior_receipts: prior,
        observed_at: time(),
        evidence: vec![],
    })
    .unwrap()
}
#[test]
fn receipt_unknown_is_not_failure_and_conflicting_observations_are_disputed() {
    let p = proposal();
    let unknown = receipt(90, ReceiptOutcome::Unknown, vec![]);
    let applied = receipt(91, ReceiptOutcome::Applied, vec![id(90)]);
    assert_eq!(
        summarize_receipts(
            p.spec().operation_id,
            p.request_digest().unwrap(),
            std::slice::from_ref(&unknown)
        )
        .unwrap(),
        ApplicationKnowledge::Unknown
    );
    assert_eq!(
        summarize_receipts(
            p.spec().operation_id,
            p.request_digest().unwrap(),
            &[unknown.clone(), applied.clone()]
        )
        .unwrap(),
        ApplicationKnowledge::Applied
    );
    let rejected = receipt(92, ReceiptOutcome::Rejected, vec![id(91)]);
    assert_eq!(
        summarize_receipts(
            p.spec().operation_id,
            p.request_digest().unwrap(),
            &[unknown, applied, rejected]
        )
        .unwrap(),
        ApplicationKnowledge::Disputed
    );
    error(
        summarize_receipts(
            p.spec().operation_id,
            Digest::of_bytes(b"changed"),
            &[receipt(90, ReceiptOutcome::Unknown, vec![])],
        ),
        ErrorCode::IdempotencyConflict,
    );
}
fn asset_snapshot() -> AssetSetSnapshot {
    AssetSetSnapshot::new(
        id(101),
        id(2),
        (0..2)
            .map(|i| AssetSetMember {
                ordinal: i,
                asset_id: id(110 + u64::from(i)),
                representations: vec![RepresentationSelection {
                    project_id: id(2),
                    asset_id: id(110 + u64::from(i)),
                    representation_id: id(120 + u64::from(i)),
                    content_revision_id: id(130 + u64::from(i)),
                    descriptor: object(b"representation"),
                }],
                metadata: vec![],
                missing_facts: vec![],
            })
            .collect(),
    )
    .unwrap()
}
fn metadata_capture() -> MetadataCapture {
    let input = asset_snapshot();
    MetadataCapture::try_from(MetadataSpec {
        input,
        field: q("exif.camera.model"),
        selector: q("example.original-v1"),
        value_type: Type::string(),
        members: (0..2)
            .map(|i| MetadataMember {
                asset_id: id(110 + i),
                representation_id: Some(id(120 + i)),
                content_revision_id: Some(id(130 + i)),
                observations: vec![object(b"observation")],
                outcome: Observation::Value(Box::new(Value::String("Camera".into()))),
            })
            .collect(),
    })
    .unwrap()
}
fn input_dep() -> Dependency {
    Dependency {
        coordinate: Coordinate::Input {
            project_id: id(2),
            graph_id: id(3),
            node_id: id(4),
            port_id: n("assets"),
        },
        projection: vec![],
    }
}
#[test]
fn metadata_common_values_distinct_preserve_disagreement_and_negative_facts() {
    let capture = metadata_capture();
    assert_eq!(
        capture.evaluate(Function::MetadataCommon).unwrap(),
        Value::String("Camera".into())
    );
    let mut s = capture.spec().clone();
    s.members[1].outcome = Observation::Value(Box::new(Value::String("Other".into())));
    let conflicting = MetadataCapture::try_from(s).unwrap();
    error(
        conflicting.evaluate(Function::MetadataCommon),
        ErrorCode::Conflict,
    );
    let values = conflicting.evaluate(Function::MetadataDistinct).unwrap();
    result_type(Function::MetadataDistinct, &Type::string())
        .unwrap()
        .check(&values)
        .unwrap();
    for (outcome, code) in [
        (Observation::Missing, ErrorCode::Unavailable),
        (Observation::Ambiguous, ErrorCode::Ambiguous),
        (Observation::StaleFingerprint, ErrorCode::StaleFingerprint),
        (Observation::Forbidden, ErrorCode::Forbidden),
        (Observation::Revoked, ErrorCode::Revoked),
    ] {
        let mut s = capture.spec().clone();
        s.members[1].outcome = outcome;
        let c = MetadataCapture::try_from(s).unwrap();
        error(c.evaluate(Function::MetadataCommon), code);
        let v = c.evaluate(Function::MetadataValues).unwrap();
        result_type(Function::MetadataValues, &Type::string())
            .unwrap()
            .check(&v)
            .unwrap();
    }
}
#[test]
fn metadata_queries_require_declared_input_and_exact_snapshot_evidence() {
    let capture = metadata_capture();
    let mut environment = env();
    environment.bindings.push(SourceBinding {
        spelling: "$input.assets".into(),
        coordinate: input_dep().coordinate,
        ty: Type::builtin(Shape::AssetSet),
    });
    environment.queries.push(QueryBinding {
        input_port: n("assets"),
        field: q("exif.camera.model"),
        selector: q("example.original-v1"),
        value_type: Type::string(),
    });
    let e = expr(
        "`metadata.common($input.assets,\"exif.camera.model\",\"example.original-v1\")`",
        FieldMode::Expression,
        environment.clone(),
        &Type::string(),
    );
    assert_eq!(e.record().dependencies.len(), 2);
    let dep = e
        .record()
        .dependencies
        .iter()
        .find(|d| matches!(d.coordinate, Coordinate::MetadataQuery { .. }))
        .unwrap()
        .clone();
    let input = fact(
        input_dep(),
        TypedValue {
            ty: Type::builtin(Shape::AssetSet),
            value: Value::AssetSet(capture.spec().input.descriptor().unwrap()),
        },
    );
    let mut query = fact(
        dep,
        TypedValue {
            ty: Type::builtin(Shape::Metadata {
                element: Box::new(Type::string()),
            }),
            value: Value::Metadata(Box::new(capture.clone())),
        },
    );
    query.dependencies = vec![input_dep()];
    assert_eq!(
        evaluate(&e, vec![input, query]).unwrap().value,
        Value::String("Camera".into())
    );
    environment.queries.clear();
    error(
        Expression::compile(
            id(9),
            FieldMode::Expression,
            &e.record().source,
            environment,
            &Type::string(),
        ),
        ErrorCode::Forbidden,
    );
    let mut s = capture.spec().clone();
    s.members[0].content_revision_id = Some(id(999));
    error(MetadataCapture::try_from(s), ErrorCode::StaleFingerprint);
}
#[test]
fn metadata_membership_is_complete_and_empty_common_is_unavailable() {
    let mut s = metadata_capture().spec().clone();
    s.members.pop();
    error(MetadataCapture::try_from(s), ErrorCode::Incomplete);
    let capture = MetadataCapture::try_from(MetadataSpec {
        input: AssetSetSnapshot::new(id(101), id(2), vec![]).unwrap(),
        field: q("exif.camera.model"),
        selector: q("example.original-v1"),
        value_type: Type::string(),
        members: vec![],
    })
    .unwrap();
    error(
        capture.evaluate(Function::MetadataCommon),
        ErrorCode::Unavailable,
    );
    assert_eq!(
        capture.evaluate(Function::MetadataValues).unwrap(),
        Value::List(vec![])
    );
}
#[test]
fn registry_and_unknown_type_versions_fail_closed() {
    let mut registry = TypeRegistry::default();
    error(
        registry.validate_value(&typed(1)),
        ErrorCode::UnsupportedVersion,
    );
    registry.register(int_type()).unwrap();
    registry.validate_value(&typed(1)).unwrap();
    error(registry.register(int_type()), ErrorCode::Conflict);
    let mut ty = int_type();
    ty.value_type.version = Version::new(2).unwrap();
    error(ty.validate(false), ErrorCode::UnsupportedVersion);
    Expression::compile_registered(
        id(9),
        FieldMode::Expression,
        "`1`",
        env(),
        &int_type(),
        &registry,
    )
    .unwrap();
}
#[test]
fn errors_never_include_source_values_paths_or_private_data() {
    let source = "`private_secret_bad_function(\"/private/user\")`";
    let error = Expression::compile(id(9), FieldMode::Expression, source, env(), &Type::string())
        .unwrap_err();
    let text = format!("{error:?} {error}");
    assert!(!text.contains("private_secret"));
    assert!(!text.contains("/private/user"));
}

#[test]
fn evaluation_context_cannot_be_reused_for_other_expression_owner_or_run() {
    let expression = expr("`1`", FieldMode::Expression, env(), &int_type());
    let snapshot = ContextSnapshot::build(spec(vec![], vec![])).unwrap();
    let context = FrozenContext::for_expression(&snapshot, &expression, None).unwrap();
    let other = expr("`2`", FieldMode::Expression, env(), &int_type());
    error(other.evaluate(&context), ErrorCode::ScopeMismatch);
    let mut environment = env();
    environment.run_id = Some(id(999));
    let other = expr("`1`", FieldMode::Expression, environment, &int_type());
    error(other.evaluate(&context), ErrorCode::ScopeMismatch);
    let mut environment = env();
    environment.owner = ScopeRef::Node {
        project_id: id(2),
        graph_id: id(3),
        node_id: id(999),
    };
    let other = expr("`1`", FieldMode::Expression, environment, &int_type());
    error(other.evaluate(&context), ErrorCode::ScopeMismatch);
}
#[test]
fn evaluated_values_retain_monotone_privacy_and_redacted_debug() {
    let mut environment = env();
    environment
        .bindings
        .push(binding(20, "private", int_type()));
    let expression = expr(
        "`$library.private+1`",
        FieldMode::Expression,
        environment,
        &int_type(),
    );
    let mut input = fact(variable_dep(20), typed(1));
    input.sensitivity = Sensitivity::Personal;
    let result = evaluate(&expression, vec![input]).unwrap();
    assert_eq!(result.sensitivity, Sensitivity::Personal);
    assert_eq!(result.value, int(2));
    assert_eq!(format!("{result:?}"), "EvaluatedValue(<redacted>)");
}
#[test]
fn source_registry_rejects_variable_workflow_values_and_contradictory_types() {
    let mut environment = env();
    environment
        .bindings
        .push(binding(20, "hidden_set", Type::builtin(Shape::AssetSet)));
    error(environment.validate(), ErrorCode::Privacy);
    let mut environment = env();
    environment.bindings = vec![
        binding(20, "a", int_type()),
        binding(20, "b", Type::string()),
    ];
    error(environment.validate(), ErrorCode::Conflict);
}
#[test]
fn source_literals_named_like_ast_fields_are_semantically_hashed() {
    let ty = Type::builtin(Shape::Record {
        fields: [
            (n("node"), int_type()),
            (n("ty"), int_type()),
            (n("span"), int_type()),
        ]
        .into(),
    });
    let first = expr(
        "`{\"node\":1,\"ty\":2,\"span\":3}`",
        FieldMode::Expression,
        env(),
        &ty,
    );
    let second = expr(
        "`{\"node\":1,\"ty\":2,\"span\":4}`",
        FieldMode::Expression,
        env(),
        &ty,
    );
    assert_ne!(first.record().ast_digest, second.record().ast_digest);
}
#[test]
fn static_collection_bounds_reject_before_evaluation() {
    error(
        Expression::compile(id(9), FieldMode::Expression, "`[1]`", env(), &int_type()),
        ErrorCode::TypeMismatch,
    );
    let ty = Type::builtin(Shape::List {
        element: Box::new(int_type()),
        max_items: 2,
    });
    error(
        Expression::compile(id(9), FieldMode::Expression, "`[1,2,3]`", env(), &ty),
        ErrorCode::LimitExceeded,
    );
    let ty = Type::builtin(Shape::List {
        element: Box::new(int_type()),
        max_items: 10_001,
    });
    error(ty.validate(true), ErrorCode::LimitExceeded);
    let value = Value::String("x".repeat(VALUE_LIMIT));
    error(Type::string().check(&value), ErrorCode::LimitExceeded);
}
#[test]
fn interpreter_operation_limit_is_enforced_without_truncation() {
    let mut environment = env();
    environment
        .bindings
        .push(binding(20, "large", Type::string()));
    let source = "`text.format(text.format($library.large,\"und\",1),\"und\",1)`";
    let expression = expr(source, FieldMode::Expression, environment, &Type::string());
    let input = fact(
        variable_dep(20),
        TypedValue {
            ty: Type::string(),
            value: Value::String("x".repeat(60_000)),
        },
    );
    error(evaluate(&expression, vec![input]), ErrorCode::LimitExceeded);
}
#[test]
fn snapshot_byte_and_expansion_limits_refuse_partial_success() {
    let entries = (0..20)
        .map(|i| {
            fact(
                variable_dep(200 + i),
                TypedValue {
                    ty: Type::string(),
                    value: Value::String("x".repeat(60_000)),
                },
            )
        })
        .collect::<Vec<_>>();
    let roots = entries.iter().map(|f| f.dependency.clone()).collect();
    error(
        ContextSnapshot::build(spec(roots, entries)),
        ErrorCode::LimitExceeded,
    );
    let entries = (0..=10_000)
        .map(|i| fact(variable_dep(200 + i), typed(1)))
        .collect::<Vec<_>>();
    error(
        ContextSnapshot::build(spec(vec![], entries)),
        ErrorCode::LimitExceeded,
    );
}
#[test]
fn variable_transitions_preserve_identity_type_claims_and_value_identity() {
    let before = variable(20, "a");
    let after = before
        .plan_clear(
            LocalRevision::INITIAL,
            RevisionCoordinate::Local {
                revision: LocalRevision::new(2).unwrap(),
            },
            time(),
        )
        .unwrap();
    validate_transition(&before, &after, LocalRevision::INITIAL, &rev()).unwrap();
    let mut changed = after.spec().clone();
    changed.value_id = Some(id(999));
    let changed = VariableAggregate::try_from(changed).unwrap();
    error(
        validate_transition(&before, &changed, LocalRevision::INITIAL, &rev()),
        ErrorCode::Conflict,
    );
    error(
        before.plan_clear(LocalRevision::INITIAL, rev(), time()),
        ErrorCode::Conflict,
    );
    let mut changed = after.spec().clone();
    changed.variable_id = id(999);
    let changed = VariableAggregate::try_from(changed).unwrap();
    error(
        validate_transition(&before, &changed, LocalRevision::INITIAL, &rev()),
        ErrorCode::TypeMismatch,
    );
}
#[test]
fn slot_capture_pins_revision_and_location_without_refreshing() {
    let dep = Dependency {
        coordinate: Coordinate::Slot {
            library_id: id(1),
            slot_id: id(80),
        },
        projection: vec![],
    };
    let mut environment = env();
    environment.bindings.push(SourceBinding {
        spelling: "$library.delivery".into(),
        coordinate: dep.coordinate.clone(),
        ty: Type::builtin(Shape::Resource),
    });
    let expression = expr(
        "`path.join($library.delivery,\"out\")`",
        FieldMode::Expression,
        environment,
        &Type::builtin(Shape::Resource),
    );
    let slot = SlotResourceValue {
        capture: photara_core::contracts::resource::SlotCapture {
            library_id: id(1),
            slot_id: id(80),
            slot_revision: LocalRevision::INITIAL,
            location_id: id(81),
        },
        components: vec![],
    };
    let input = fact(
        dep,
        TypedValue {
            ty: Type::builtin(Shape::Resource),
            value: Value::SlotResource(slot),
        },
    );
    let result = evaluate(&expression, vec![input.clone()]).unwrap();
    let Value::SlotResource(value) = result.value else {
        panic!()
    };
    assert_eq!(value.capture.location_id, id(81));
    assert_eq!(value.components, vec!["out"]);
    let mut stale = input;
    stale.revision = RevisionCoordinate::Local {
        revision: LocalRevision::new(2).unwrap(),
    };
    error(stale.validate(), ErrorCode::Stale);
}
#[test]
fn temp_context_keys_are_run_bound_even_if_host_reuses_a_lease_id() {
    let fact = DeviceFact {
        place: HostPlace::Temp,
        binding_id: id(51),
        generation: 1,
        availability: DeviceAvailability::Ready,
        temp_lease: Some(id(52)),
    };
    assert_ne!(
        DeviceContextSnapshot::build(id(11), id(5), vec![fact.clone()])
            .unwrap()
            .digest(),
        DeviceContextSnapshot::build(id(11), id(6), vec![fact])
            .unwrap()
            .digest()
    );
}
#[test]
fn receipt_dispute_does_not_skip_validation_of_later_evidence() {
    let proposal = proposal();
    let mut bad = receipt(94, ReceiptOutcome::Unknown, vec![]).spec().clone();
    bad.request_digest = Digest::of_bytes(b"changed");
    let bad = ApplyReceipt::try_from(bad).unwrap();
    error(
        summarize_receipts(
            proposal.spec().operation_id,
            proposal.request_digest().unwrap(),
            &[
                receipt(90, ReceiptOutcome::Applied, vec![]),
                receipt(91, ReceiptOutcome::Rejected, vec![id(90)]),
                bad,
            ],
        ),
        ErrorCode::IdempotencyConflict,
    );
}
#[test]
fn arithmetic_and_formatting_are_deterministic_across_repeated_compilation() {
    for left in -10..=10 {
        for right in -10..=10 {
            let source = format!("`({left})+({right})*2`");
            let a = expr(&source, FieldMode::Expression, env(), &int_type());
            let b = expr(&source, FieldMode::Expression, env(), &int_type());
            assert_eq!(a.record().ast_digest, b.record().ast_digest);
            assert_eq!(evaluate(&a, vec![]).unwrap().value, int(left + right * 2));
        }
    }
}
fn golden() -> serde_json::Value {
    let mut environment = env();
    environment
        .bindings
        .push(binding(10_010, "delivery_count", int_type()));
    let expression = expr(
        "Count: `text.format($library.delivery_count + 1,\"und\",1)`",
        FieldMode::Template,
        environment,
        &Type::string(),
    );
    let input = fact(variable_dep(10_010), typed(6));
    let snapshot =
        ContextSnapshot::build(spec(expression.record().dependencies.clone(), vec![input]))
            .unwrap();
    let result = expression
        .evaluate(&FrozenContext::for_expression(&snapshot, &expression, None).unwrap())
        .unwrap();
    let key = NodeCacheKey::build(
        &key_spec(expression.record().dependencies.clone()),
        &snapshot,
        None,
    )
    .unwrap();
    json!({"schema":"photara.d19-context.v1","codec":"photara.canonical-json.v1","expression":expression.record(),"snapshot":snapshot,"result":{"value":result.value,"sensitivity":result.sensitivity,"portability":result.portability,"context_digest":result.context_digest},"node_cache_key":key,"graph_cache_key":graph_cache_key(id(3),&[(id(4),key.digest())],&snapshot,Digest::of_bytes(b"environment"),None,&boundary()).unwrap()})
}
fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/fixtures/generation-two/d19-context.json")
}
#[test]
#[ignore = "explicit generation only; never rewrites existing CXT1a/S6/D18 bytes"]
fn generate_d19_context_golden() {
    std::fs::write(
        fixture_path(),
        photara_core::canonical_json(&golden()).unwrap(),
    )
    .unwrap();
}
#[test]
fn rust_generated_context_golden_matches_exact_canonical_bytes() {
    let bytes = std::fs::read(fixture_path()).expect("generate new context addendum");
    assert_eq!(bytes, photara_core::canonical_json(&golden()).unwrap());
}
#[test]
fn run_override_capture_cannot_be_replayed_as_another_runs_override() {
    let variable = variable(20, "a");
    let override_value = RunOverride {
        override_id: id(40),
        run_id: id(5),
        target: variable.dependency(),
        expected_revision: LocalRevision::INITIAL,
        value: typed(8),
        sensitivity: Sensitivity::Ordinary,
        portability: Portability::Portable,
    };
    let resolved = resolve_variables(&[variable], &[], &[override_value], id(5)).unwrap();
    let snapshot =
        ContextSnapshot::build(spec(vec![variable_dep(20)], vec![resolved[0].fact.clone()]))
            .unwrap();
    let mut environment = env();
    environment.bindings.push(binding(20, "a", int_type()));
    let expression = expr(
        "`$library.a`",
        FieldMode::Expression,
        environment.clone(),
        &int_type(),
    );
    let context = FrozenContext::for_expression(&snapshot, &expression, None).unwrap();
    assert_eq!(expression.evaluate(&context).unwrap().value, int(8));
    environment.run_id = Some(id(6));
    let expression = expr(
        "`$library.a`",
        FieldMode::Expression,
        environment,
        &int_type(),
    );
    error(
        FrozenContext::for_expression(&snapshot, &expression, None),
        ErrorCode::ScopeMismatch,
    );
}
#[test]
fn device_cache_keys_accept_explicit_host_dependencies_only_on_device() {
    let mut captured = spec(vec![], vec![]);
    captured.device_required = true;
    let snapshot = ContextSnapshot::build(captured).unwrap();
    let device = DeviceContextSnapshot::build(
        id(11),
        id(5),
        vec![DeviceFact {
            place: HostPlace::Home,
            binding_id: id(50),
            generation: 1,
            availability: DeviceAvailability::Ready,
            temp_lease: None,
        }],
    )
    .unwrap();
    let mut key = key_spec(vec![Dependency {
        coordinate: Coordinate::HostPlace {
            place: HostPlace::Home,
        },
        projection: vec![],
    }]);
    key.location = CacheLocation::Device;
    let result = NodeCacheKey::build(&key, &snapshot, Some(&device)).unwrap();
    result
        .authorize_lookup(&boundary(), true, Some(device.digest()))
        .unwrap();
    key.location = CacheLocation::Project;
    error(
        NodeCacheKey::build(&key, &snapshot, Some(&device)),
        ErrorCode::Privacy,
    );
}
