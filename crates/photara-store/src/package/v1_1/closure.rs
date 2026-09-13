use super::{
    ObjectRef, PackageError, Value, array, checked, ct, cx, decode, field, kind, known, records,
    reference, text,
};
fn one(v: &Value, k: &str, out: &mut Vec<ObjectRef>) -> Result<(), PackageError> {
    out.push(reference(v, k)?);
    Ok(())
}
fn optional(v: &Value, k: &str, out: &mut Vec<ObjectRef>) -> Result<(), PackageError> {
    if v.get(k).is_some_and(|x| !x.is_null()) {
        one(v, k, out)?;
    }
    Ok(())
}
fn many(v: &Value, k: &str, out: &mut Vec<ObjectRef>) -> Result<(), PackageError> {
    for r in array(v, k)? {
        out.push(decode(r)?);
    }
    Ok(())
}
fn binding(v: &Value, out: &mut Vec<ObjectRef>) -> Result<(), PackageError> {
    if v.is_null() {
        return Ok(());
    }
    match text(v, "kind")? {
        "expression" => one(v, "value", out),
        "literal" => typed(field(v, "value")?, out),
        _ => Err(PackageError::Record),
    }
}
pub(super) fn typed(v: &Value, out: &mut Vec<ObjectRef>) -> Result<(), PackageError> {
    let t: cx::value::TypedValue = decode(v)?;
    checked(t.validate(false))?;
    value(&t.value, out)
}
fn core_ref(r: &ct::schema::ObjectRef, out: &mut Vec<ObjectRef>) -> Result<(), PackageError> {
    out.push(decode(
        &serde_json::to_value(r).map_err(|_| PackageError::Record)?,
    )?);
    Ok(())
}
pub(super) fn value(v: &cx::value::Value, out: &mut Vec<ObjectRef>) -> Result<(), PackageError> {
    use cx::value::Value as V;
    match v {
        V::Resource(r) => match &r.descriptor {
            ct::resource::ResourceDescriptor::Managed { resource } => {
                core_ref(&resource.spec().blob, out)?;
            }
            ct::resource::ResourceDescriptor::HostPlace { .. } => return Err(PackageError::Record),
            _ => {}
        },
        V::List(xs) => {
            for x in xs {
                value(x, out)?;
            }
        }
        V::Record(xs) => {
            for x in xs.values() {
                value(x, out)?;
            }
        }
        V::Optional(Some(x)) => value(x, out)?,
        V::Metadata(m) => {
            for member in m.spec().input.members() {
                for r in &member.representations {
                    core_ref(&r.descriptor, out)?;
                }
                for m in &member.metadata {
                    core_ref(&m.object, out)?;
                }
            }
            for member in &m.spec().members {
                for r in &member.observations {
                    core_ref(r, out)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
#[expect(
    clippy::too_many_lines,
    reason = "Explicit field-by-field closure avoids inspecting arbitrary JSON"
)]
pub(in super::super) fn references(
    v: &Value,
    out: &mut Vec<ObjectRef>,
) -> Result<(), PackageError> {
    if !known(v)? {
        return super::super::records::references(v, out);
    }
    match kind(v)?.0 {
        "photara.project.authored" => {
            for k in [
                "party_assignments",
                "location_assignments",
                "asset_ledger",
                "resource_ledger",
                "context",
            ] {
                one(v, k, out)?;
            }
            optional(field(v, "origin")?, "source_object", out)?;
            for g in array(v, "graphs")? {
                one(g, "document", out)?;
            }
        }
        "photara.project.saved-graph" => {
            one(v, "context", out)?;
            for n in array(v, "node_contracts")? {
                optional(n, "manifest", out)?;
            }
            for p in array(v, "required_packages")? {
                optional(p, "manifest", out)?;
            }
        }
        "photara.project.asset-ledger" => {
            for a in array(v, "assets")? {
                for r in array(a, "representations")? {
                    one(r, "current_content", out)?;
                    if r.get("retained_content").is_some() {
                        many(r, "retained_content", out)?;
                    }
                }
            }
        }
        "photara.project.resource-ledger" => {
            for k in ["managed_resources", "external_resources", "artifacts"] {
                many(v, k, out)?;
            }
        }
        "photara.project.managed-resource" => one(v, "blob", out)?,
        "photara.project.external-resource"
        | "photara.project.library-snapshot"
        | "photara.node.manifest" => {}
        "photara.project.representation-content" => {
            let b = field(v, "binding")?;
            one(
                b,
                if text(b, "kind")? == "managed" {
                    "version"
                } else {
                    "revision"
                },
                out,
            )?;
            for l in array(v, "lineage")? {
                optional(l, "descriptor", out)?;
            }
        }
        "photara.value.asset-set-snapshot" => {
            for p in array(v, "pages")? {
                one(p, "page", out)?;
            }
        }
        "photara.value.asset-set-page" => {
            for m in array(v, "members")? {
                for r in array(m, "representations")? {
                    one(r, "descriptor", out)?;
                }
                many(m, "metadata_refs", out)?;
            }
        }
        "photara.context.authored" => {
            for k in [
                "variables",
                "expressions",
                "captures",
                "metadata_selections",
            ] {
                many(v, k, out)?;
            }
            for n in array(v, "node_contexts")? {
                one(n, "context", out)?;
            }
        }
        "photara.context.variable" => {
            binding(field(v, "default")?, out)?;
            let cur = field(v, "current_value")?;
            if !cur.is_null() {
                binding(field(cur, "binding")?, out)?;
            }
        }
        "photara.context.expression" => {
            // The AST is verified by recompilation. Its literal types select any resource edges.
            fn ast(a: &cx::expression::Ast, out: &mut Vec<ObjectRef>) -> Result<(), PackageError> {
                use cx::expression::AstKind as A;
                match &a.node {
                    A::Literal { value: v } => value(v, out)?,
                    A::Binary { left, right, .. } => {
                        ast(left, out)?;
                        ast(right, out)?;
                    }
                    A::Unary { value, .. } | A::Field { value, .. } => ast(value, out)?,
                    A::If {
                        condition,
                        then_value,
                        else_value,
                    } => {
                        ast(condition, out)?;
                        ast(then_value, out)?;
                        ast(else_value, out)?;
                    }
                    A::Call {
                        arguments: args, ..
                    }
                    | A::Query {
                        arguments: args, ..
                    }
                    | A::Template { parts: args }
                    | A::List { items: args } => {
                        for a in args {
                            ast(a, out)?;
                        }
                    }
                    A::Record { fields } => {
                        for a in fields.values() {
                            ast(a, out)?;
                        }
                    }
                    A::Reference { .. } => {}
                }
                Ok(())
            }
            ast(&records::expression(v)?.record().ast, out)?;
        }
        "photara.context.snapshot" => {
            for f in &records::snapshot(v)?.spec().entries {
                if let cx::snapshot::FactValue::Present(x) = &f.value {
                    value(x, out)?;
                }
            }
            for k in ["captures", "expressions"] {
                many(v, k, out)?;
            }
        }
        "photara.metadata.observation" => typed(field(v, "value")?, out)?,
        "photara.metadata.patch" => {
            one(v, "input_snapshot", out)?;
            optional(field(v, "target")?, "group_snapshot", out)?;
            for op in array(v, "operations")? {
                if text(op, "kind")? != "remove" {
                    typed(field(op, "value")?, out)?;
                }
                for p in array(op, "preconditions")? {
                    one(p, "descriptor", out)?;
                }
            }
        }
        "photara.context.change-proposal" => {
            one(v, "snapshot", out)?;
            typed(field(v, "literal")?, out)?;
        }
        "photara.context.apply-receipt" | "photara.history.receipt" => many(v, "evidence", out)?,
        "photara.value.group-set" => one(v, "input_snapshot", out)?,
        "photara.history.artifact" => {
            one(v, "descriptor", out)?;
            many(v, "evidence", out)?;
        }
        "photara.project.history" => {
            for r in array(v, "runs")? {
                one(r, "document", out)?;
            }
            for k in [
                "operations",
                "evidence",
                "snapshots",
                "metadata_observations",
                "proposals",
                "apply_receipts",
                "artifacts",
            ] {
                many(v, k, out)?;
            }
        }
        "photara.history.run-start" => {
            one(v, "source_authored", out)?;
            one(v, "context_snapshot", out)?;
            many(v, "dependencies", out)?;
            for i in array(v, "input_snapshots")? {
                one(i, "snapshot", out)?;
            }
        }
        "photara.history.attempt-start" => {
            for k in [
                "representation_revisions",
                "input_snapshot_refs",
                "operation_refs",
            ] {
                many(v, k, out)?;
            }
        }
        "photara.history.effect-intent" => typed(field(v, "request")?, out)?,
        _ => return Err(PackageError::UnsupportedVersion),
    }
    Ok(())
}
