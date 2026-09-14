use crate::CompileError;
use crate::expression_parser;
use crate::handler_types::StaticType;
use language_core::{Expr, Program};
use std::collections::HashMap;
#[cfg(test)]
pub(super) fn parse_expr(input: &str, program: &Program) -> Result<Expr, CompileError> {
    expression_parser::parse_expr_in_namespace(input, "", program)
}
pub(super) fn parse_expr_in_namespace(
    input: &str,
    namespace: &str,
    program: &Program,
) -> Result<Expr, CompileError> {
    expression_parser::parse_expr_in_namespace(input, namespace, program)
}
pub(super) fn validate_expr(
    e: &Expr,
    k: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<(), CompileError> {
    match e {
        Expr::Variable(n) => {
            if !k.contains_key(n) {
                return Err(CompileError::UnknownVariable(n.clone()));
            }
        }
        Expr::Field { base, field } => match k.get(base) {
            Some(StaticType::Model(model_type)) => {
                let model = p
                    .model(&model_type.name)
                    .ok_or_else(|| CompileError::UnknownModel(model_type.name.clone()))?;
                if !model.fields.iter().any(|f| f.name == *field) {
                    return Err(CompileError::Syntax(format!(
                        "model `{}` has no field `{field}`",
                        model_type.name
                    )));
                }
            }
            Some(StaticType::Upload) => {
                if !matches!(
                    field.as_str(),
                    "path" | "filename" | "contentType" | "bytes"
                ) {
                    return Err(CompileError::Syntax(format!(
                        "Upload has no field `{field}`"
                    )));
                }
            }
            Some(value) if value.is_scalar(language_core::ValueType::Image) => {
                if !matches!(
                    field.as_str(),
                    "path" | "contentType" | "width" | "height" | "bytes"
                ) {
                    return Err(CompileError::Syntax(format!(
                        "Image has no field `{field}`"
                    )));
                }
            }
            Some(StaticType::PureStruct(schema)) => {
                let def = p.json_schema(schema).ok_or_else(|| {
                    CompileError::Syntax(format!("unknown pure struct `{schema}`"))
                })?;
                if !def.fields.iter().any(|f| f.name == *field) {
                    return Err(CompileError::Syntax(format!(
                        "struct `{schema}` has no field `{field}`"
                    )));
                }
            }
            _ => {
                return Err(CompileError::Syntax(format!(
                    "`{base}` is not a model/upload value"
                )));
            }
        },
        Expr::Slugify(inner) => {
            validate_expr(inner, k, p)?;
        }
        Expr::Builtin { args, .. } => {
            for arg in args {
                validate_expr(arg, k, p)?;
            }
        }
        Expr::Not(inner) => validate_expr(inner, k, p)?,
        Expr::Binary { left, right, .. } => {
            validate_expr(left, k, p)?;
            validate_expr(right, k, p)?
        }
        Expr::F32ArrayNew { len, fill } => {
            validate_expr(len, k, p)?;
            validate_expr(fill, k, p)?;
        }
        Expr::CollectionIndex { index, .. } => validate_expr(index, k, p)?,
        Expr::CollectionLen { .. } => {}
        Expr::PureSumPredicate { .. } | Expr::PureSumUnwrapOr { .. } => {
            crate::sum_expression::validate(e, k, p)?
        }
        _ => {}
    }
    Ok(())
}
pub(super) use crate::expression_security::infer_static_expr_type;
pub(super) use crate::expression_type::infer_expr_type;
