use crate::builtin_types;
use crate::diagnostics::CompileError;
use crate::handler_types::StaticType;
use crate::type_semantics::represented_as;
use language_core::{BinaryOp, Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) fn infer_expr_type(
    e: &Expr,
    k: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<ValueType, CompileError> {
    match e {
        Expr::String(_) => Ok(ValueType::String),
        Expr::Int(_) => Ok(ValueType::Int),
        Expr::F32(_) => Ok(ValueType::F32),
        Expr::F32ArrayNew { len, fill } => {
            if infer_expr_type(len, k, p)? != ValueType::Int
                || infer_expr_type(fill, k, p)? != ValueType::F32
            {
                return Err(CompileError::Syntax(
                    "arrayF32(len, fill) requires Int and F32".into(),
                ));
            }
            Ok(ValueType::F32Array)
        }
        Expr::CollectionIndex { collection, index } => match k.get(collection) {
            Some(value) if value.is_scalar(ValueType::F32Array) => {
                if infer_expr_type(index, k, p)? != ValueType::Int {
                    return Err(CompileError::Syntax("Array<F32> index must be Int".into()));
                }
                Ok(ValueType::F32)
            }
            Some(value) if value.is_scalar(ValueType::StringList) => {
                if infer_expr_type(index, k, p)? != ValueType::Int {
                    return Err(CompileError::Syntax(
                        "List<String> index must be Int".into(),
                    ));
                }
                Ok(ValueType::String)
            }
            Some(value) if value.is_scalar(ValueType::StringDict) => {
                if infer_expr_type(index, k, p)? != ValueType::String {
                    return Err(CompileError::Syntax(
                        "Dict<String,String> key must be String".into(),
                    ));
                }
                Ok(ValueType::String)
            }
            _ => Err(CompileError::Syntax(format!(
                "`{collection}` is not an indexable collection"
            ))),
        },
        Expr::CollectionLen { collection } => {
            let supports_len = k
                .get(collection)
                .and_then(StaticType::scalar)
                .map(|scalar| {
                    matches!(
                        p.representation_type(scalar.value_type)
                            .unwrap_or(scalar.value_type),
                        ValueType::String
                            | ValueType::F32Array
                            | ValueType::StringList
                            | ValueType::StringDict
                    )
                })
                .unwrap_or(false);
            if supports_len {
                Ok(ValueType::Int)
            } else {
                Err(CompileError::Syntax(format!(
                    "`{collection}` does not support len(...)"
                )))
            }
        }
        Expr::Builtin { function, args } => {
            builtin_types::infer_builtin_type(*function, args, k, p)
        }
        Expr::Bool(_) => Ok(ValueType::Bool),
        Expr::EnumLiteral { enum_id, .. } => Ok(ValueType::Enum(*enum_id)),
        Expr::Slugify(inner) => {
            if !represented_as(p, infer_expr_type(inner, k, p)?, ValueType::String) {
                return Err(CompileError::Syntax(
                    "slug(...) requires a String expression".into(),
                ));
            }
            Ok(ValueType::Slug)
        }
        Expr::Variable(n) => match k.get(n) {
            Some(StaticType::Scalar(t)) => Ok(t.value_type),
            Some(StaticType::Upload) => Err(CompileError::Syntax(format!(
                "Upload `{n}` cannot be interpolated directly; use a metadata field"
            ))),
            Some(StaticType::Model(_)) => Err(CompileError::Syntax(format!(
                "model `{n}` cannot be interpolated directly; use a field"
            ))),
            Some(StaticType::OptionalModel(_)) => Err(CompileError::Syntax(format!(
                "optional model `{n}` must be checked with `@if` before field access"
            ))),
            Some(StaticType::ListModel(_)) => Err(CompileError::Syntax(format!(
                "list `{n}` must be iterated with `@for`"
            ))),
            Some(StaticType::PureStruct(_)) => Err(CompileError::Syntax(format!(
                "struct `{n}` cannot be used as a scalar directly; access a field"
            ))),
            Some(StaticType::PureOption(_)) => Err(CompileError::Syntax(format!(
                "Option value `{n}` is opaque until a safe sum-type method such as `.is_some()` or `.unwrap_or(...)` is used"
            ))),
            Some(StaticType::PureResult { .. }) => Err(CompileError::Syntax(format!(
                "Result value `{n}` is opaque until a safe sum-type method such as `.is_ok()` or `.unwrap_or(...)` is used"
            ))),
            None => Err(CompileError::UnknownVariable(n.clone())),
        },
        Expr::Field { base, field } => match k.get(base) {
            Some(StaticType::Model(model_type)) => p
                .model(&model_type.name)
                .and_then(|x| x.fields.iter().find(|f| f.name == *field))
                .map(|f| f.ty)
                .ok_or_else(|| CompileError::Syntax(format!("unknown field `{base}.{field}`"))),
            Some(StaticType::Upload) => match field.as_str() {
                "path" | "filename" | "contentType" => Ok(ValueType::String),
                "bytes" => Ok(ValueType::Int),
                _ => Err(CompileError::Syntax(format!(
                    "Upload has no field `{field}`"
                ))),
            },
            Some(value) if value.is_scalar(ValueType::Image) => match field.as_str() {
                "path" | "contentType" => Ok(ValueType::String),
                "width" | "height" | "bytes" => Ok(ValueType::Int),
                _ => Err(CompileError::Syntax(format!(
                    "Image has no field `{field}`"
                ))),
            },
            Some(StaticType::PureStruct(schema)) => p
                .json_schema(schema)
                .and_then(|x| x.fields.iter().find(|f| f.name == *field))
                .map(|f| f.ty)
                .ok_or_else(|| CompileError::Syntax(format!("unknown field `{base}.{field}`"))),
            _ => Err(CompileError::Syntax(format!(
                "`{base}` is not a model/upload/pure struct"
            ))),
        },
        Expr::PureSumPredicate { base, predicate } => {
            crate::sum_expression::infer_predicate_type(base, *predicate, k)
        }
        Expr::PureSumUnwrapOr { base, fallback } => {
            crate::sum_expression::infer_unwrap_or_type(base, fallback, k, p)
        }
        Expr::Not(inner) => {
            if infer_expr_type(inner, k, p)? == ValueType::Bool {
                Ok(ValueType::Bool)
            } else {
                Err(CompileError::Syntax("logical ! requires Bool".into()))
            }
        }
        Expr::Binary { left, op, right } => {
            let l = infer_expr_type(left, k, p)?;
            let r = infer_expr_type(right, k, p)?;
            match op {
                BinaryOp::Add if l == ValueType::String && r == ValueType::String => {
                    Ok(ValueType::String)
                }
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
                    if l == ValueType::Int && r == ValueType::Int =>
                {
                    Ok(ValueType::Int)
                }
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
                    if l == ValueType::F32 && r == ValueType::F32 =>
                {
                    Ok(ValueType::F32)
                }
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
                    if l == ValueType::Decimal && r == ValueType::Decimal =>
                {
                    Ok(ValueType::Decimal)
                }
                BinaryOp::ShiftLeft
                | BinaryOp::ShiftRight
                | BinaryOp::BitAnd
                | BinaryOp::BitXor
                | BinaryOp::BitOr
                    if l == ValueType::Int && r == ValueType::Int =>
                {
                    Ok(ValueType::Int)
                }
                BinaryOp::LogicalAnd | BinaryOp::LogicalOr
                    if l == ValueType::Bool && r == ValueType::Bool =>
                {
                    Ok(ValueType::Bool)
                }
                BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
                    if l == r
                        && matches!(
                            p.representation_type(l).unwrap_or(l),
                            ValueType::Int | ValueType::F32 | ValueType::Decimal
                        ) =>
                {
                    Ok(ValueType::Bool)
                }
                BinaryOp::Eq | BinaryOp::Ne
                    if l == r
                        && matches!(
                            p.representation_type(l).unwrap_or(l),
                            ValueType::String
                                | ValueType::Int
                                | ValueType::F32
                                | ValueType::Decimal
                                | ValueType::Bool
                        ) =>
                {
                    Ok(ValueType::Bool)
                }
                _ => Err(CompileError::Syntax("invalid binary operands".into())),
            }
        }
    }
}
