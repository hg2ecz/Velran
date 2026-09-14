use crate::diagnostics::CompileError;
use crate::expression::{infer_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::StaticType;
use crate::source_syntax::is_identifier;
use language_core::{Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) fn parse_string_dict_set(
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
            "{handler_kind} `{handler_name}` dictionary set target must be dict[key]"
        ))
    })?;
    let close = lhs.rfind(']').ok_or_else(|| {
        CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` dictionary set target missing ]"
        ))
    })?;
    if close != lhs.len() - 1 {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` invalid Dict<String,String> set target"
        )));
    }
    let dict = lhs[..open].trim();
    if !is_identifier(dict)
        || !known
            .get(dict)
            .map(|value| value.is_scalar(ValueType::StringDict))
            .unwrap_or(false)
    {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` set target `{dict}` is not Dict<String,String>"
        )));
    }
    let key = parse_expr_in_namespace(lhs[open + 1..close].trim(), namespace, program)?;
    let value = parse_expr_in_namespace(rhs.trim(), namespace, program)?;
    validate_expr(&key, known, program)?;
    validate_expr(&value, known, program)?;
    if infer_expr_type(&key, known, program)? != ValueType::String
        || infer_expr_type(&value, known, program)? != ValueType::String
    {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` Dict<String,String> set requires String key and String value"
        )));
    }
    Ok((dict.into(), key, value))
}
