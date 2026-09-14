use crate::diagnostics::CompileError;
use crate::expression::{infer_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::StaticType;
use crate::source_syntax::{find_statement_end, is_identifier};
use language_core::{Program, Statement, ValueType};
use std::collections::HashMap;

pub(super) fn parse(
    page: &str,
    namespace: &str,
    body: &str,
    cursor: usize,
    allow_resource: bool,
    preceding: &[Statement],
    known: &HashMap<String, StaticType>,
    program: &mut Program,
) -> Result<(Statement, usize), CompileError> {
    require_top_level(page, allow_resource, preceding)?;
    let end = find_statement_end(body, cursor)?;
    let text = body[cursor..end].trim().trim_end_matches(';').trim();
    let rest = text.strip_prefix("canonical slug ").ok_or_else(|| {
        CompileError::Syntax(format!("page `{page}` invalid canonical slug syntax"))
    })?;
    let (param, expr_text) = rest.split_once(" from ").ok_or_else(|| {
        CompileError::Syntax(format!(
            "page `{page}` canonical slug syntax is `canonical slug <path-param> from <Slug expression>`"
        ))
    })?;
    let param = param.trim();
    require_slug_param(page, param, known)?;
    let canonical = parse_expr_in_namespace(expr_text.trim(), namespace, program)?;
    validate_expr(&canonical, known, program)?;
    if infer_expr_type(&canonical, known, program)? != ValueType::Slug {
        return Err(CompileError::Syntax(format!(
            "page `{page}` canonical slug expression must have type Slug"
        )));
    }
    Ok((
        Statement::CanonicalSlug {
            param: param.into(),
            canonical,
        },
        end + 1,
    ))
}

fn require_top_level(
    page: &str,
    allow_resource: bool,
    preceding: &[Statement],
) -> Result<(), CompileError> {
    if !allow_resource {
        return Err(CompileError::Syntax(format!(
            "page `{page}` canonical slug must be a top-level page statement"
        )));
    }
    if preceding
        .iter()
        .any(|statement| matches!(statement, Statement::CanonicalSlug { .. }))
    {
        return Err(CompileError::Syntax(format!(
            "page `{page}` supports exactly one canonical slug invariant in v0.1"
        )));
    }
    Ok(())
}

fn require_slug_param(
    page: &str,
    param: &str,
    known: &HashMap<String, StaticType>,
) -> Result<(), CompileError> {
    if !is_identifier(param) {
        return Err(CompileError::Syntax(format!(
            "page `{page}` canonical slug path parameter is invalid"
        )));
    }
    match known.get(param) {
        Some(value) if value.is_scalar(ValueType::Slug) => Ok(()),
        _ => Err(CompileError::Syntax(format!(
            "page `{page}` canonical slug parameter `{param}` must have type Slug"
        ))),
    }
}
