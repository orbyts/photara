/// Evaluated value retains its privacy labels and exact frozen dependency provenance.
/// No serializer is provided; persistence uses validated captures or literal proposals.
pub struct EvaluatedValue {
    pub ty: super::value::Type,
    pub value: Value,
    pub sensitivity: super::snapshot::Sensitivity,
    pub portability: super::snapshot::Portability,
    pub expression_digest: crate::contracts::schema::Digest,
    pub context_digest: crate::contracts::schema::Digest,
}
impl std::fmt::Debug for EvaluatedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EvaluatedValue(<redacted>)")
    }
}
use super::expression::{Ast, AstKind, Binary, Expression, Function, Unary};
use super::snapshot::FrozenContext;
use super::value::{Integer, TypedValue, Value};
use super::{ErrorCode, OPERATIONS, Result, VALUE_LIMIT};
use std::collections::BTreeMap;
impl Expression {
    pub fn evaluate(&self, context: &FrozenContext) -> Result<EvaluatedValue> {
        context.validate_for(self)?;
        // Preflight every declared read for access and exact type; both branches remain dependencies.
        for dep in &self.record().dependencies {
            context.get(dep)?;
        }
        let mut budget = OPERATIONS;
        let value = evaluate(&self.record().ast, context, &mut budget)?;
        let result = TypedValue {
            ty: self.record().ast.ty.clone(),
            value,
        };
        result.validate(false)?;
        Ok(EvaluatedValue {
            ty: result.ty,
            value: result.value,
            sensitivity: context.sensitivity(),
            portability: context.portability(),
            expression_digest: self.record().ast_digest,
            context_digest: context.dependency_digest()?,
        })
    }
}
fn charge(budget: &mut usize, n: usize) -> Result<()> {
    *budget = budget.checked_sub(n).ok_or(ErrorCode::LimitExceeded)?;
    Ok(())
}
fn evaluate(ast: &Ast, c: &FrozenContext, budget: &mut usize) -> Result<Value> {
    charge(budget, 1)?;
    let result = match &ast.node {
        AstKind::Literal { value } => value.clone(),
        AstKind::Reference { dependency } => {
            let v = c.get(dependency)?;
            if v.ty != ast.ty {
                return Err(ErrorCode::TypeMismatch.into());
            }
            v.value
        }
        AstKind::Unary { operator, value } => match (operator, evaluate(value, c, budget)?) {
            (Unary::Not, Value::Bool(b)) => Value::Bool(!b),
            (Unary::Negate, Value::Integer(n)) => Value::Integer(Integer::new(
                n.get().checked_neg().ok_or(ErrorCode::Overflow)?,
            )),
            _ => return Err(ErrorCode::TypeMismatch.into()),
        },
        AstKind::Binary {
            operator,
            left,
            right,
        } => {
            let l = evaluate(left, c, budget)?;
            if *operator == Binary::And && l == Value::Bool(false) {
                Value::Bool(false)
            } else if *operator == Binary::Or && l == Value::Bool(true) {
                Value::Bool(true)
            } else {
                let r = evaluate(right, c, budget)?;
                binary(*operator, l, r)?
            }
        }
        AstKind::If {
            condition,
            then_value,
            else_value,
        } => match evaluate(condition, c, budget)? {
            Value::Bool(true) => evaluate(then_value, c, budget)?,
            Value::Bool(false) => evaluate(else_value, c, budget)?,
            _ => return Err(ErrorCode::TypeMismatch.into()),
        },
        AstKind::Template { parts } => {
            let mut text = String::new();
            for p in parts {
                let Value::String(s) = evaluate(p, c, budget)? else {
                    return Err(ErrorCode::TypeMismatch.into());
                };
                charge(budget, s.chars().count())?;
                if text.len() + s.len() > VALUE_LIMIT {
                    return Err(ErrorCode::LimitExceeded.into());
                }
                text.push_str(&s);
            }
            Value::String(text)
        }
        AstKind::List { items } => {
            let mut values = Vec::new();
            for a in items {
                values.push(evaluate(a, c, budget)?);
            }
            Value::List(values)
        }
        AstKind::Record { fields } => {
            let mut values = BTreeMap::new();
            for (k, a) in fields {
                values.insert(k.clone(), evaluate(a, c, budget)?);
            }
            Value::Record(values)
        }
        AstKind::Field { value, field } => {
            let Value::Record(mut fields) = evaluate(value, c, budget)? else {
                return Err(ErrorCode::TypeMismatch.into());
            };
            fields.remove(field).ok_or(ErrorCode::Unknown)?
        }
        AstKind::Query { .. } | AstKind::Call { .. } => evaluate_call(&ast.node, c, budget)?,
    };
    ast.ty.check(&result).map_err(|mut e| {
        e.span = Some(ast.span);
        e
    })?;
    Ok(result)
}
fn evaluate_call(node: &AstKind, c: &FrozenContext, budget: &mut usize) -> Result<Value> {
    Ok(match node {
        AstKind::Query {
            function,
            arguments,
            dependency,
        } => {
            let Value::AssetSet(input) = evaluate(&arguments[0], c, budget)? else {
                return Err(ErrorCode::TypeMismatch.into());
            };
            let capture = c.get(dependency)?;
            let Value::Metadata(metadata) = capture.value else {
                return Err(ErrorCode::TypeMismatch.into());
            };
            let super::expression::Coordinate::MetadataQuery {
                field, selector, ..
            } = &dependency.coordinate
            else {
                return Err(ErrorCode::ScopeMismatch.into());
            };
            if metadata.spec().input.descriptor()? != input
                || &metadata.spec().field != field
                || &metadata.spec().selector != selector
            {
                return Err(ErrorCode::StaleFingerprint.into());
            }
            charge(budget, metadata.spec().members.len())?;
            metadata.evaluate(*function)?
        }
        AstKind::Call {
            function,
            arguments,
        } => {
            if *function == Function::OptionalOrElse {
                match evaluate(&arguments[0], c, budget)? {
                    Value::Optional(Some(v)) => *v,
                    Value::Optional(None) => evaluate(&arguments[1], c, budget)?,
                    _ => return Err(ErrorCode::TypeMismatch.into()),
                }
            } else {
                let mut a = Vec::new();
                for v in arguments {
                    a.push(evaluate(v, c, budget)?);
                }
                call(*function, &a, budget)?
            }
        }
        _ => return Err(ErrorCode::UnsupportedVersion.into()),
    })
}
fn binary(op: Binary, l: Value, r: Value) -> Result<Value> {
    if matches!(op, Binary::Add | Binary::Subtract | Binary::Multiply) {
        let (Value::Integer(l), Value::Integer(r)) = (l, r) else {
            return Err(ErrorCode::TypeMismatch.into());
        };
        return Ok(Value::Integer(Integer::new(
            match op {
                Binary::Add => l.get().checked_add(r.get()),
                Binary::Subtract => l.get().checked_sub(r.get()),
                _ => l.get().checked_mul(r.get()),
            }
            .ok_or(ErrorCode::Overflow)?,
        )));
    }
    if matches!(op, Binary::Equal | Binary::NotEqual) {
        return Ok(Value::Bool(if op == Binary::Equal {
            l == r
        } else {
            l != r
        }));
    }
    if matches!(op, Binary::And | Binary::Or) {
        let (Value::Bool(l), Value::Bool(r)) = (l, r) else {
            return Err(ErrorCode::TypeMismatch.into());
        };
        return Ok(Value::Bool(if op == Binary::And { l && r } else { l || r }));
    }
    let order = match (l, r) {
        (Value::Integer(l), Value::Integer(r)) => l.cmp(&r),
        (Value::String(l), Value::String(r)) => l.cmp(&r),
        (Value::Timestamp(l), Value::Timestamp(r)) => l.cmp(&r),
        (Value::Decimal(l), Value::Decimal(r)) if l.scale == r.scale => l
            .coefficient
            .parse::<i128>()
            .map_err(|_| ErrorCode::Overflow)?
            .cmp(
                &r.coefficient
                    .parse::<i128>()
                    .map_err(|_| ErrorCode::Overflow)?,
            ),
        _ => return Err(ErrorCode::TypeMismatch.into()),
    };
    Ok(Value::Bool(match op {
        Binary::Less => order.is_lt(),
        Binary::LessEqual => order.is_le(),
        Binary::Greater => order.is_gt(),
        Binary::GreaterEqual => order.is_ge(),
        _ => return Err(ErrorCode::TypeMismatch.into()),
    }))
}
fn call(function: Function, a: &[Value], budget: &mut usize) -> Result<Value> {
    match function {
        Function::TextFormat => {
            if a.get(1) != Some(&Value::String("und".into()))
                || a.get(2) != Some(&Value::Integer(Integer::new(1)))
            {
                return Err(ErrorCode::UnsupportedVersion.into());
            }
            let s = match &a[0] {
                Value::Bool(v) => v.to_string(),
                Value::String(v) => v.clone(),
                Value::Integer(v) => v.get().to_string(),
                Value::Decimal(v) => v.formatted(),
                Value::Timestamp(v) => v.as_str().into(),
                Value::Enum(v) => v.as_str().into(),
                _ => return Err(ErrorCode::TypeMismatch.into()),
            };
            charge(budget, s.chars().count())?;
            Ok(Value::String(s))
        }
        Function::PathJoin => {
            let components = a[1..]
                .iter()
                .map(|v| {
                    if let Value::String(s) = v {
                        Ok(s.clone())
                    } else {
                        Err(ErrorCode::TypeMismatch.into())
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            match &a[0] {
                Value::Resource(root) => Ok(Value::Resource(Box::new(root.join(&components)?))),
                Value::SlotResource(root) => Ok(Value::SlotResource(root.join(&components)?)),
                _ => Err(ErrorCode::TypeMismatch.into()),
            }
        }
        Function::AssetsCount => {
            let Value::AssetSet(s) = &a[0] else {
                return Err(ErrorCode::TypeMismatch.into());
            };
            Ok(Value::Integer(Integer::new(i64::from(s.member_count))))
        }
        _ => Err(ErrorCode::UnsupportedVersion.into()),
    }
}
