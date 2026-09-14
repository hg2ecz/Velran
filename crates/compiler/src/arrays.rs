use crate::diagnostics::CompileError;
use crate::expression::{infer_expr_type, parse_expr_in_namespace};
use crate::handler_types::StaticType;
use crate::source_syntax::is_identifier;
use language_core::{Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) fn parse_f32_array_set(
    handler_kind: &str,
    handler_name: &str,
    text: &str,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(String, Expr, Expr), CompileError> {
    let (lhs, rhs) = text.split_once('=').ok_or_else(|| {
        CompileError::Syntax(format!("{handler_kind} `{handler_name}` set requires ="))
    })?;
    let lhs = lhs.trim();
    let open = lhs.find('[').ok_or_else(|| {
        CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` set target must be array[index]"
        ))
    })?;
    let close = lhs.rfind(']').ok_or_else(|| {
        CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` set target missing ]"
        ))
    })?;
    if close != lhs.len() - 1 {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` invalid Array<F32> set target"
        )));
    }
    let array = lhs[..open].trim();
    if !is_identifier(array)
        || !known
            .get(array)
            .map(|value| value.is_scalar(ValueType::F32Array))
            .unwrap_or(false)
    {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` set target `{array}` is not Array<F32>"
        )));
    }
    let index = parse_expr_in_namespace(lhs[open + 1..close].trim(), namespace, program)?;
    let value = parse_expr_in_namespace(rhs.trim(), namespace, program)?;
    if infer_expr_type(&index, known, program)? != ValueType::Int
        || infer_expr_type(&value, known, program)? != ValueType::F32
    {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` Array<F32> set requires Int index and F32 value"
        )));
    }
    Ok((array.into(), index, value))
}
