//! Versioned ID-bound expressions. Only compilation creates executable expressions.
use super::value::{Shape, Type, Value};
use super::{
    AST_DEPTH, AST_NODES, DIRECT_DEPENDENCIES, ErrorCode, Result, SOURCE_LIMIT, Span, VALUE_LIMIT,
    bytes, digest,
};
use crate::contracts::{
    dto::{RevisionCoordinate, ScopeRef},
    ids::{
        AssetId, AssetRepresentationId, ContentRevisionId, ExpressionId, GraphId, LibraryId,
        NodeInstanceId, ProjectId, RunId, StorageSlotId, VariableId,
    },
    resource::HostPlace,
    schema::{Digest, LocalName, Version},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FieldMode {
    LiteralOnly,
    Expression,
    Template,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Coordinate {
    Variable {
        scope: ScopeRef,
        variable_id: VariableId,
    },
    Slot {
        library_id: LibraryId,
        slot_id: StorageSlotId,
    },
    Input {
        project_id: ProjectId,
        graph_id: GraphId,
        node_id: NodeInstanceId,
        port_id: LocalName,
    },
    MetadataQuery {
        project_id: ProjectId,
        graph_id: GraphId,
        node_id: NodeInstanceId,
        port_id: LocalName,
        field: crate::contracts::schema::QualifiedName,
        selector: crate::contracts::schema::QualifiedName,
    },
    AssetMetadata {
        project_id: ProjectId,
        graph_id: GraphId,
        node_id: NodeInstanceId,
        asset_id: AssetId,
        representation_id: AssetRepresentationId,
        content_revision_id: ContentRevisionId,
        field: crate::contracts::schema::QualifiedName,
    },
    ProjectRoot {
        project_id: ProjectId,
    },
    ProjectArtifacts {
        project_id: ProjectId,
    },
    RunOverrides {
        project_id: ProjectId,
        run_id: RunId,
    },
    HostPlace {
        place: HostPlace,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub coordinate: Coordinate,
    pub projection: Vec<LocalName>,
}
impl Dependency {
    pub fn key(&self) -> Result<String> {
        String::from_utf8(bytes(self, VALUE_LIMIT)?)
            .map_err(|_| ErrorCode::InvalidCoordinate.into())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBinding {
    pub spelling: String,
    pub coordinate: Coordinate,
    pub ty: Type,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingEnvironment {
    pub owner: ScopeRef,
    pub owning_library_id: LibraryId,
    pub owner_revision: RevisionCoordinate,
    pub run_id: Option<RunId>,
    pub asset_id: Option<AssetId>,
    pub bindings: Vec<SourceBinding>,
    pub queries: Vec<super::metadata::QueryBinding>,
}
#[must_use]
pub const fn project(scope: ScopeRef) -> Option<ProjectId> {
    match scope {
        ScopeRef::Library { .. } => None,
        ScopeRef::Project { project_id }
        | ScopeRef::Graph { project_id, .. }
        | ScopeRef::Node { project_id, .. } => Some(project_id),
    }
}
#[must_use]
pub fn scope_allows(owner: ScopeRef, target: ScopeRef, library_id: LibraryId) -> bool {
    match target {
        ScopeRef::Library {
            library_id: target_library,
        } => target_library == library_id,
        ScopeRef::Project { project_id } => project(owner) == Some(project_id),
        ScopeRef::Graph {
            project_id,
            graph_id,
        } => {
            matches!(owner,ScopeRef::Graph{project_id:owner_project,graph_id:owner_graph}|ScopeRef::Node{project_id:owner_project,graph_id:owner_graph,..} if owner_project==project_id && owner_graph==graph_id)
        }
        ScopeRef::Node { .. } => owner == target,
    }
}
impl BindingEnvironment {
    pub fn validate(&self) -> Result<()> {
        if self.bindings.len() > super::EXPANDED_DEPENDENCIES {
            return Err(ErrorCode::LimitExceeded.into());
        }
        if let ScopeRef::Library { library_id } = self.owner
            && library_id != self.owning_library_id
        {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        let mut queries = std::collections::BTreeSet::new();
        for q in &self.queries {
            q.value_type.validate(true)?;
            if !matches!(self.owner, ScopeRef::Node { .. })
                || !queries.insert(bytes(
                    &(q.input_port.clone(), q.field.clone(), q.selector.clone()),
                    VALUE_LIMIT,
                )?)
            {
                return Err(ErrorCode::Conflict.into());
            }
        }
        if self.queries.len() > 256 {
            return Err(ErrorCode::LimitExceeded.into());
        }
        let mut names = std::collections::BTreeSet::new();
        let mut coordinates = BTreeMap::new();
        for b in &self.bindings {
            b.ty.validate(matches!(
                b.coordinate,
                Coordinate::Variable { .. }
                    | Coordinate::AssetMetadata { .. }
                    | Coordinate::RunOverrides { .. }
            ))?;
            let coordinate = Dependency {
                coordinate: b.coordinate.clone(),
                projection: vec![],
            }
            .key()?;
            if coordinates
                .insert(coordinate, &b.ty)
                .is_some_and(|old| old != &b.ty)
            {
                return Err(ErrorCode::Conflict.into());
            }
            if matches!(
                b.coordinate,
                Coordinate::Slot { .. }
                    | Coordinate::ProjectRoot { .. }
                    | Coordinate::ProjectArtifacts { .. }
                    | Coordinate::HostPlace { .. }
            ) && b.ty.shape != Shape::Resource
            {
                return Err(ErrorCode::TypeMismatch.into());
            }
            self.authorize_coordinate(&b.coordinate)?;
            if !names.insert(&b.spelling) {
                return Err(ErrorCode::Conflict.into());
            }
            if !valid_spelling(b) {
                return Err(ErrorCode::InvalidCoordinate.into());
            }
        }
        Ok(())
    }
    pub fn authorize_coordinate(&self, c: &Coordinate) -> Result<()> {
        let allowed = match c {
            Coordinate::Variable { scope, .. } => {
                scope_allows(self.owner, *scope, self.owning_library_id)
            }
            Coordinate::Slot { library_id, .. } => *library_id == self.owning_library_id,
            Coordinate::MetadataQuery {
                project_id,
                graph_id,
                node_id,
                ..
            }
            | Coordinate::Input {
                project_id,
                graph_id,
                node_id,
                ..
            } => {
                self.owner
                    == ScopeRef::Node {
                        project_id: *project_id,
                        graph_id: *graph_id,
                        node_id: *node_id,
                    }
            }
            Coordinate::AssetMetadata {
                project_id,
                graph_id,
                node_id,
                asset_id,
                ..
            } => {
                if self.asset_id != Some(*asset_id) {
                    return Err(ErrorCode::AssetContextRequired.into());
                }
                self.owner
                    == ScopeRef::Node {
                        project_id: *project_id,
                        graph_id: *graph_id,
                        node_id: *node_id,
                    }
            }
            Coordinate::ProjectRoot { project_id }
            | Coordinate::ProjectArtifacts { project_id } => {
                project(self.owner) == Some(*project_id)
            }
            Coordinate::RunOverrides { project_id, run_id } => {
                matches!(self.owner, ScopeRef::Node { .. })
                    && project(self.owner) == Some(*project_id)
                    && self.run_id == Some(*run_id)
            }
            Coordinate::HostPlace { .. } => matches!(self.owner, ScopeRef::Node { .. }),
        };
        if allowed {
            Ok(())
        } else {
            Err(ErrorCode::ScopeMismatch.into())
        }
    }
    fn bind(&self, spelling: &str) -> Result<(Dependency, Type)> {
        let b = self
            .bindings
            .iter()
            .filter(|b| {
                spelling == b.spelling
                    || spelling
                        .strip_prefix(&b.spelling)
                        .is_some_and(|s| s.starts_with('.'))
            })
            .max_by_key(|b| b.spelling.len())
            .ok_or_else(|| {
                super::ContextError::from(
                    if spelling.starts_with("$asset.") && self.asset_id.is_none() {
                        ErrorCode::AssetContextRequired
                    } else {
                        ErrorCode::Unknown
                    },
                )
            })?;
        let tail = &spelling[b.spelling.len()..];
        let mut ty = b.ty.clone();
        let mut projection = Vec::new();
        if !tail.is_empty() {
            for part in tail[1..].split('.') {
                let field = LocalName::parse(part)?;
                if let Shape::Record { fields } = &ty.shape {
                    ty = fields.get(&field).cloned().ok_or(ErrorCode::Unknown)?;
                } else {
                    return Err(ErrorCode::TypeMismatch.into());
                }
                projection.push(field);
            }
        }
        Ok((
            Dependency {
                coordinate: b.coordinate.clone(),
                projection,
            },
            ty,
        ))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Binary {
    Add,
    Subtract,
    Multiply,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Unary {
    Not,
    Negate,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Function {
    TextFormat,
    OptionalOrElse,
    PathJoin,
    AssetsCount,
    MetadataValues,
    MetadataCommon,
    MetadataDistinct,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum AstKind {
    Literal {
        value: Value,
    },
    Reference {
        dependency: Dependency,
    },
    Binary {
        operator: Binary,
        left: Box<Ast>,
        right: Box<Ast>,
    },
    Unary {
        operator: Unary,
        value: Box<Ast>,
    },
    If {
        condition: Box<Ast>,
        then_value: Box<Ast>,
        else_value: Box<Ast>,
    },
    Query {
        function: Function,
        arguments: Vec<Ast>,
        dependency: Dependency,
    },
    Call {
        function: Function,
        arguments: Vec<Ast>,
    },
    Template {
        parts: Vec<Ast>,
    },
    List {
        items: Vec<Ast>,
    },
    Record {
        fields: BTreeMap<LocalName, Ast>,
    },
    Field {
        value: Box<Ast>,
        field: LocalName,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ast {
    pub span: Span,
    pub ty: Type,
    pub node: AstKind,
}
impl Ast {
    fn semantic(&self) -> Result<serde_json::Value> {
        use serde_json::json;
        let list = |values: &[Ast]| values.iter().map(Ast::semantic).collect::<Result<Vec<_>>>();
        let node = match &self.node {
            AstKind::Literal { value } => json!({"kind":"literal","value":value}),
            AstKind::Reference { dependency } => {
                json!({"kind":"reference","dependency":dependency})
            }
            AstKind::Unary { operator, value } => {
                json!({"kind":"unary","operator":operator,"value":value.semantic()?})
            }
            AstKind::Binary {
                operator,
                left,
                right,
            } => {
                json!({"kind":"binary","operator":operator,"left":left.semantic()?,"right":right.semantic()?})
            }
            AstKind::If {
                condition,
                then_value,
                else_value,
            } => {
                json!({"kind":"if","condition":condition.semantic()?,"then_value":then_value.semantic()?,"else_value":else_value.semantic()?})
            }
            AstKind::Call {
                function,
                arguments,
            } => json!({"kind":"call","function":function,"arguments":list(arguments)?}),
            AstKind::Query {
                function,
                arguments,
                dependency,
            } => {
                json!({"kind":"query","function":function,"arguments":list(arguments)?,"dependency":dependency})
            }
            AstKind::Template { parts } => json!({"kind":"template","parts":list(parts)?}),
            AstKind::List { items } => json!({"kind":"list","items":list(items)?}),
            AstKind::Record { fields } => {
                json!({"kind":"record","fields":fields.iter().map(|(k,v)|Ok((k.clone(),v.semantic()?))).collect::<Result<BTreeMap<_,_>>>()?})
            }
            AstKind::Field { value, field } => {
                json!({"kind":"field","value":value.semantic()?,"field":field})
            }
        };
        Ok(json!({"ty":self.ty,"node":node}))
    }
    pub fn dependencies(&self) -> Result<Vec<Dependency>> {
        let mut pending = vec![self];
        let mut found = BTreeMap::new();
        while let Some(a) = pending.pop() {
            match &a.node {
                AstKind::Reference { dependency } => {
                    found.insert(dependency.key()?, dependency.clone());
                }
                AstKind::Binary { left, right, .. } => {
                    pending.push(left);
                    pending.push(right);
                }
                AstKind::Unary { value, .. } | AstKind::Field { value, .. } => pending.push(value),
                AstKind::If {
                    condition,
                    then_value,
                    else_value,
                } => {
                    pending.push(condition);
                    pending.push(then_value);
                    pending.push(else_value);
                }
                AstKind::Query {
                    arguments,
                    dependency,
                    ..
                } => {
                    found.insert(dependency.key()?, dependency.clone());
                    pending.extend(arguments);
                }
                AstKind::Call { arguments, .. } => pending.extend(arguments),
                AstKind::Template { parts } => pending.extend(parts),
                AstKind::List { items } => pending.extend(items),
                AstKind::Record { fields } => pending.extend(fields.values()),
                AstKind::Literal { .. } => {}
            }
        }
        if found.len() > DIRECT_DEPENDENCIES {
            return Err(ErrorCode::LimitExceeded.into());
        }
        Ok(found.into_values().collect())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionRecord {
    pub expression_id: ExpressionId,
    pub language_version: Version,
    pub compiler_version: Version,
    pub ast_version: Version,
    pub mode: FieldMode,
    pub environment: BindingEnvironment,
    pub source: String,
    pub source_sha256: Digest,
    pub ast: Ast,
    pub ast_digest: Digest,
    pub dependencies: Vec<Dependency>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Expression(ExpressionRecord);
impl Expression {
    pub fn compile(
        id: ExpressionId,
        mode: FieldMode,
        source: &str,
        environment: BindingEnvironment,
        expected: &Type,
    ) -> Result<Self> {
        if source.len() > SOURCE_LIMIT {
            return Err(ErrorCode::LimitExceeded.into());
        }
        if mode == FieldMode::LiteralOnly {
            return Err(ErrorCode::UnsupportedVersion.into());
        }
        environment.validate()?;
        expected.validate(false)?;
        let raw = super::parser::parse(mode, source)?;
        let mut count = 0;
        let ast = type_ast(raw, &environment, Some(expected), 0, &mut count)?;
        if &ast.ty != expected {
            return Err(super::at(
                ErrorCode::TypeMismatch,
                ast.span.start,
                ast.span.end,
            ));
        }
        bytes(&ast, VALUE_LIMIT)?;
        let dependencies = ast.dependencies()?;
        let ast_digest = digest(
            &serde_json::json!({"domain":"photara.expression-ast.v1","ast":ast.semantic()?}),
            VALUE_LIMIT,
        )?;
        Ok(Self(ExpressionRecord {
            expression_id: id,
            language_version: Version::FIRST,
            compiler_version: Version::FIRST,
            ast_version: Version::FIRST,
            mode,
            environment,
            source: source.into(),
            source_sha256: Digest::of_bytes(source.as_bytes()),
            ast,
            ast_digest,
            dependencies,
        }))
    }
    /// Compiles against an explicit host registry of exact schema/type associations.
    pub fn compile_registered(
        id: ExpressionId,
        mode: FieldMode,
        source: &str,
        environment: BindingEnvironment,
        expected: &Type,
        registry: &super::value::TypeRegistry,
    ) -> Result<Self> {
        registry.validate_type(expected)?;
        for b in &environment.bindings {
            registry.validate_type(&b.ty)?;
        }
        for q in &environment.queries {
            registry.validate_type(&q.value_type)?;
        }
        Self::compile(id, mode, source, environment, expected)
    }
    pub fn verify(record: ExpressionRecord) -> Result<Self> {
        if record.language_version != Version::FIRST
            || record.compiler_version != Version::FIRST
            || record.ast_version != Version::FIRST
        {
            return Err(ErrorCode::UnsupportedVersion.into());
        }
        let compiled = Self::compile(
            record.expression_id,
            record.mode,
            &record.source,
            record.environment.clone(),
            &record.ast.ty,
        )?;
        if compiled.0 != record {
            return Err(ErrorCode::SourceAstMismatch.into());
        }
        Ok(Self(record))
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        Self::verify(crate::contracts::schema::decode_strict(
            bytes,
            super::SNAPSHOT_LIMIT,
        )?)
    }
    #[must_use]
    pub fn record(&self) -> &ExpressionRecord {
        &self.0
    }
}

pub(crate) fn type_ast(
    raw: super::parser::Raw,
    env: &BindingEnvironment,
    expected: Option<&Type>,
    depth: usize,
    count: &mut usize,
) -> Result<Ast> {
    let span = raw.span;
    type_ast_inner(raw, env, expected, depth, count).map_err(|mut e| {
        if e.span.is_none() {
            e.span = Some(span);
        }
        e
    })
}
// Keep the bounded grammar dispatch exhaustive and auditable in one match.
#[allow(clippy::too_many_lines)]
fn type_ast_inner(
    raw: super::parser::Raw,
    env: &BindingEnvironment,
    expected: Option<&Type>,
    depth: usize,
    count: &mut usize,
) -> Result<Ast> {
    use super::parser::RawKind;
    *count += 1;
    if depth >= AST_DEPTH || *count > AST_NODES {
        return Err(ErrorCode::LimitExceeded.into());
    }
    let span = raw.span;
    let typed = |r: super::parser::Raw, expected: Option<&Type>, count: &mut usize| {
        type_ast(r, env, expected, depth + 1, count)
    };
    let (ty, node) = match raw.kind {
        RawKind::Literal(value) => {
            let inferred = match &value {
                Value::Bool(_) => Type::builtin(Shape::Bool),
                Value::String(_) => Type::string(),
                Value::Integer(_) => Type::builtin(Shape::Integer),
                Value::Null => Type::builtin(Shape::Null),
                _ => return Err(ErrorCode::UnsupportedVersion.into()),
            };
            let ty = expected.unwrap_or(&inferred).clone();
            ty.check(&value)?;
            (ty, AstKind::Literal { value })
        }
        RawKind::Reference(name) => {
            let (dependency, ty) = env.bind(&name).map_err(|mut e| {
                e.span = Some(span);
                e
            })?;
            (ty, AstKind::Reference { dependency })
        }
        RawKind::Binary(operator, left, right) => {
            let left = typed(*left, None, count)?;
            let right = typed(*right, Some(&left.ty), count)?;
            if left.ty != right.ty {
                return Err(super::at(ErrorCode::TypeMismatch, span.start, span.end));
            }
            let ty = match operator {
                Binary::Add | Binary::Subtract | Binary::Multiply
                    if left.ty.shape == Shape::Integer =>
                {
                    left.ty.clone()
                }
                Binary::And | Binary::Or if left.ty.shape == Shape::Bool => {
                    Type::builtin(Shape::Bool)
                }
                Binary::Equal | Binary::NotEqual
                    if !matches!(
                        left.ty.shape,
                        Shape::AssetSet | Shape::Resource | Shape::Metadata { .. }
                    ) =>
                {
                    Type::builtin(Shape::Bool)
                }
                Binary::Less | Binary::LessEqual | Binary::Greater | Binary::GreaterEqual
                    if matches!(
                        left.ty.shape,
                        Shape::Integer
                            | Shape::Decimal { .. }
                            | Shape::Timestamp
                            | Shape::String { .. }
                    ) =>
                {
                    Type::builtin(Shape::Bool)
                }
                _ => return Err(super::at(ErrorCode::TypeMismatch, span.start, span.end)),
            };
            (
                ty,
                AstKind::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            )
        }
        RawKind::Unary(operator, value) => {
            let value = typed(*value, None, count)?;
            if !matches!(
                (operator, &value.ty.shape),
                (Unary::Not, Shape::Bool) | (Unary::Negate, Shape::Integer)
            ) {
                return Err(ErrorCode::TypeMismatch.into());
            }
            (
                value.ty.clone(),
                AstKind::Unary {
                    operator,
                    value: Box::new(value),
                },
            )
        }
        RawKind::If(condition, then_value, else_value) => {
            let condition = typed(*condition, Some(&Type::builtin(Shape::Bool)), count)?;
            if condition.ty.shape != Shape::Bool {
                return Err(ErrorCode::TypeMismatch.into());
            }
            let then_value = typed(*then_value, expected, count)?;
            let else_value = typed(*else_value, Some(&then_value.ty), count)?;
            if then_value.ty != else_value.ty {
                return Err(ErrorCode::TypeMismatch.into());
            }
            (
                then_value.ty.clone(),
                AstKind::If {
                    condition: Box::new(condition),
                    then_value: Box::new(then_value),
                    else_value: Box::new(else_value),
                },
            )
        }
        RawKind::Template(parts) => {
            let mut out = Vec::new();
            for p in parts {
                let a = typed(p, Some(&Type::string()), count)?;
                if !matches!(a.ty.shape, Shape::String { .. }) {
                    return Err(ErrorCode::TypeMismatch.into());
                }
                out.push(a);
            }
            (Type::string(), AstKind::Template { parts: out })
        }
        RawKind::List(items) => {
            if expected.is_some_and(|t| !matches!(t.shape, Shape::List { .. })) {
                return Err(ErrorCode::TypeMismatch.into());
            }
            if let Some(Type {
                shape: Shape::List { max_items, .. },
                ..
            }) = expected
                && items.len() > *max_items
            {
                return Err(ErrorCode::LimitExceeded.into());
            }
            let expected_element = expected.and_then(|t| {
                if let Shape::List { element, .. } = &t.shape {
                    Some(element.as_ref())
                } else {
                    None
                }
            });
            let mut out = Vec::new();
            for r in items {
                let a = typed(
                    r,
                    expected_element.or_else(|| out.first().map(|a: &Ast| &a.ty)),
                    count,
                )?;
                out.push(a);
            }
            let element = expected_element
                .cloned()
                .or_else(|| out.first().map(|a| a.ty.clone()))
                .ok_or(ErrorCode::TypeMismatch)?;
            if out.iter().any(|a| a.ty != element) {
                return Err(ErrorCode::TypeMismatch.into());
            }
            let ty = expected.cloned().unwrap_or_else(|| {
                Type::builtin(Shape::List {
                    element: Box::new(element),
                    max_items: 10_000,
                })
            });
            (ty, AstKind::List { items: out })
        }
        RawKind::Record(fields) => {
            let mut out = BTreeMap::new();
            for (k, r) in fields {
                let e = expected.and_then(|t| {
                    if let Shape::Record { fields } = &t.shape {
                        fields.get(&k)
                    } else {
                        None
                    }
                });
                out.insert(k, typed(r, e, count)?);
            }
            let shape = Shape::Record {
                fields: out.iter().map(|(k, a)| (k.clone(), a.ty.clone())).collect(),
            };
            let ty = expected
                .cloned()
                .unwrap_or_else(|| Type::builtin(shape.clone()));
            if ty.shape != shape {
                return Err(ErrorCode::TypeMismatch.into());
            }
            (ty, AstKind::Record { fields: out })
        }
        RawKind::Field(value, field) => {
            let value = typed(*value, None, count)?;
            let Shape::Record { fields } = &value.ty.shape else {
                return Err(ErrorCode::TypeMismatch.into());
            };
            let ty = fields.get(&field).cloned().ok_or(ErrorCode::Unknown)?;
            (
                ty,
                AstKind::Field {
                    value: Box::new(value),
                    field,
                },
            )
        }
        RawKind::Call(name, args) => {
            let mut arguments = Vec::new();
            for a in args {
                arguments.push(typed(a, None, count)?);
            }
            if matches!(
                name.as_str(),
                "metadata.values" | "metadata.common" | "metadata.distinct"
            ) {
                let (function, ty, dependency) = query_type(&name, &arguments, env)?;
                (
                    ty,
                    AstKind::Query {
                        function,
                        arguments,
                        dependency,
                    },
                )
            } else {
                let (function, ty) = call_type(&name, &arguments)?;
                (
                    ty,
                    AstKind::Call {
                        function,
                        arguments,
                    },
                )
            }
        }
    };
    Ok(Ast { span, ty, node })
}
fn call_type(name: &str, a: &[Ast]) -> Result<(Function, Type)> {
    let string = |a: &Ast| matches!(a.ty.shape, Shape::String { .. });
    let out = match name {
        "text.format"
            if a.len() == 3
                && string(&a[1])
                && a[2].ty.shape == Shape::Integer
                && matches!(
                    a[0].ty.shape,
                    Shape::Bool
                        | Shape::String { .. }
                        | Shape::Integer
                        | Shape::Decimal { .. }
                        | Shape::Timestamp
                        | Shape::Enum { .. }
                ) =>
        {
            (Function::TextFormat, Type::string())
        }
        "optional.or_else" if a.len() == 2 => {
            let Shape::Optional { element } = &a[0].ty.shape else {
                return Err(ErrorCode::TypeMismatch.into());
            };
            if **element != a[1].ty {
                return Err(ErrorCode::TypeMismatch.into());
            }
            (Function::OptionalOrElse, *element.clone())
        }
        "path.join"
            if a.len() >= 2 && a[0].ty.shape == Shape::Resource && a[1..].iter().all(string) =>
        {
            (Function::PathJoin, a[0].ty.clone())
        }
        "assets.count" if a.len() == 1 && a[0].ty.shape == Shape::AssetSet => {
            (Function::AssetsCount, Type::builtin(Shape::Integer))
        }
        "metadata.values" | "metadata.common" | "metadata.distinct"
            if a.len() == 3
                && a[0].ty.shape == Shape::AssetSet
                && string(&a[1])
                && string(&a[2]) =>
        {
            // Query result is an exact declared captured input, bound by the compiler below.
            return Err(ErrorCode::UnsupportedVersion.into());
        }
        "text.format" | "optional.or_else" | "path.join" | "assets.count" => {
            return Err(ErrorCode::TypeMismatch.into());
        }
        _ => return Err(ErrorCode::Unknown.into()),
    };
    Ok(out)
}

fn query_type(
    name: &str,
    args: &[Ast],
    env: &BindingEnvironment,
) -> Result<(Function, Type, Dependency)> {
    if args.len() != 3 || args[0].ty.shape != Shape::AssetSet {
        return Err(ErrorCode::TypeMismatch.into());
    }
    let AstKind::Reference { dependency: input } = &args[0].node else {
        return Err(ErrorCode::TypeMismatch.into());
    };
    let Coordinate::Input {
        project_id,
        graph_id,
        node_id,
        port_id,
    } = &input.coordinate
    else {
        return Err(ErrorCode::ScopeMismatch.into());
    };
    if !input.projection.is_empty() {
        return Err(ErrorCode::ScopeMismatch.into());
    }
    let (
        AstKind::Literal {
            value: Value::String(field),
        },
        AstKind::Literal {
            value: Value::String(selector),
        },
    ) = (&args[1].node, &args[2].node)
    else {
        return Err(ErrorCode::TypeMismatch.into());
    };
    let q = env
        .queries
        .iter()
        .find(|q| {
            &q.input_port == port_id && q.field.as_str() == field && q.selector.as_str() == selector
        })
        .ok_or(ErrorCode::Forbidden)?;
    let function = match name {
        "metadata.values" => Function::MetadataValues,
        "metadata.common" => Function::MetadataCommon,
        _ => Function::MetadataDistinct,
    };
    let ty = super::metadata::result_type(function, &q.value_type)?;
    Ok((
        function,
        ty,
        Dependency {
            coordinate: Coordinate::MetadataQuery {
                project_id: *project_id,
                graph_id: *graph_id,
                node_id: *node_id,
                port_id: port_id.clone(),
                field: q.field.clone(),
                selector: q.selector.clone(),
            },
            projection: vec![],
        },
    ))
}

fn valid_spelling(b: &SourceBinding) -> bool {
    match &b.coordinate {
        Coordinate::MetadataQuery { .. } => false,
        Coordinate::Variable { scope, .. } => {
            let prefix = match scope {
                ScopeRef::Library { .. } => "$library.",
                ScopeRef::Project { .. } => "$project.",
                ScopeRef::Graph { .. } => "$graph.",
                ScopeRef::Node { .. } => "$node.",
            };
            b.spelling.strip_prefix(prefix).is_some_and(|s| {
                LocalName::parse(s).is_ok()
                    && !matches!(
                        s,
                        "root" | "artifacts" | "overrides" | "metadata" | "assets"
                    )
            })
        }
        Coordinate::Slot { .. } => b
            .spelling
            .strip_prefix("$library.")
            .is_some_and(|s| LocalName::parse(s).is_ok()),
        Coordinate::Input { port_id, .. } => b.spelling == format!("$input.{}", port_id.as_str()),
        Coordinate::AssetMetadata { field, .. } => {
            b.spelling == format!("$asset.metadata.{}", field.as_str())
        }
        Coordinate::ProjectRoot { .. } => b.spelling == "$project.root",
        Coordinate::ProjectArtifacts { .. } => b.spelling == "$project.artifacts",
        Coordinate::RunOverrides { .. } => b.spelling == "$run.overrides",
        Coordinate::HostPlace { place } => {
            b.spelling
                == format!(
                    "${}",
                    match place {
                        HostPlace::Home => "HOME",
                        HostPlace::Downloads => "DOWNLOADS",
                        HostPlace::Desktop => "DESKTOP",
                        HostPlace::Documents => "DOCUMENTS",
                        HostPlace::Pictures => "PICTURES",
                        HostPlace::Temp => "TEMP",
                    }
                )
        }
    }
}

/// Field admission keeps literal-only data out of expression records and their persistence schema.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum CompiledField {
    Literal(Box<super::value::TypedValue>),
    Expression(Box<Expression>),
}
impl CompiledField {
    pub fn compile(
        id: ExpressionId,
        mode: FieldMode,
        source: &str,
        environment: BindingEnvironment,
        expected: &Type,
    ) -> Result<Self> {
        if mode == FieldMode::LiteralOnly {
            if source.len() > SOURCE_LIMIT {
                return Err(ErrorCode::LimitExceeded.into());
            }
            let value = super::value::TypedValue {
                ty: expected.clone(),
                value: Value::String(source.into()),
            };
            value.validate(true)?;
            Ok(Self::Literal(Box::new(value)))
        } else {
            Ok(Self::Expression(Box::new(Expression::compile(
                id,
                mode,
                source,
                environment,
                expected,
            )?)))
        }
    }
}
