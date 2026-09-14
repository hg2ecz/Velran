use crate::CompileError;
use crate::expression::infer_expr_type;
use crate::handler_types::StaticType;
use language_core::{Expr, Program, PureSumPredicate, PureValueType, ValueType};
use std::collections::HashMap;

pub(super) fn infer_predicate_type(
    base: &str,
    predicate: PureSumPredicate,
    known: &HashMap<String, StaticType>,
) -> Result<ValueType, CompileError> {
    use PureSumPredicate::*;
    match (known.get(base), predicate) {
        (Some(StaticType::PureOption(_)), IsSome | IsNone)
        | (Some(StaticType::PureResult { .. }), IsOk | IsErr) => Ok(ValueType::Bool),
        (Some(StaticType::PureOption(_)), IsOk | IsErr) => Err(CompileError::Syntax(format!(
            "Option value `{base}` supports `.is_some()`/`.is_none()`, not Result predicates"
        ))),
        (Some(StaticType::PureResult { .. }), IsSome | IsNone) => Err(CompileError::Syntax(
            format!("Result value `{base}` supports `.is_ok()`/`.is_err()`, not Option predicates"),
        )),
        _ => Err(CompileError::Syntax(format!(
            "`{base}` is not a compatible Option/Result value"
        ))),
    }
}

pub(super) fn infer_unwrap_or_type(
    base: &str,
    fallback: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ValueType, CompileError> {
    let expected = match known.get(base) {
        Some(StaticType::PureOption(inner)) => inner,
        Some(StaticType::PureResult { ok, .. }) => ok,
        _ => {
            return Err(CompileError::Syntax(format!(
                "`{base}` is not an Option/Result value for `.unwrap_or(...)`"
            )));
        }
    };
    let expected_ty = value_type(expected)?;
    let actual = infer_expr_type(fallback, known, program)?;
    if actual != expected_ty {
        return Err(CompileError::Syntax(format!(
            "`.unwrap_or(...)` fallback type mismatch for `{base}`"
        )));
    }
    Ok(expected_ty)
}

fn value_type(value: &PureValueType) -> Result<ValueType, CompileError> {
    match value {
        PureValueType::Int => Ok(ValueType::Int),
        PureValueType::F32 => Ok(ValueType::F32),
        PureValueType::Bool => Ok(ValueType::Bool),
        PureValueType::String => Ok(ValueType::String),
        PureValueType::StringList => Ok(ValueType::StringList),
        PureValueType::Struct(_) => Err(CompileError::Syntax(
            "`.unwrap_or(...)` for struct payloads is not enabled yet".into(),
        )),
    }
}

pub(super) fn validate(
    expr: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    match expr {
        Expr::PureSumPredicate { base, .. } => {
            if !known.contains_key(base) {
                return Err(CompileError::UnknownVariable(base.clone()));
            }
        }
        Expr::PureSumUnwrapOr { base, fallback } => {
            if !known.contains_key(base) {
                return Err(CompileError::UnknownVariable(base.clone()));
            }
            crate::expression::validate_expr(fallback, known, program)?;
        }
        _ => unreachable!("sum expression validator contract"),
    }
    Ok(())
}
