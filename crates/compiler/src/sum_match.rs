use crate::diagnostics::CompileError;
use crate::expression::{infer_static_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::StaticType;
use crate::source_syntax::{matching_brace, read_ident, skip_ws_and_comments};
use language_core::{Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) struct ParsedMatchArm {
    pub variant: String,
    pub body: String,
    pub body_offset: usize,
}

pub(super) struct ParsedMatch {
    pub expr: Expr,
    pub enum_id: u16,
    pub arms: Vec<ParsedMatchArm>,
    pub close: usize,
}

pub(super) fn parse_match(
    handler_kind: &str,
    handler_name: &str,
    namespace: &str,
    body: &str,
    cursor: usize,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ParsedMatch, CompileError> {
    let expr_start = cursor + "match ".len();
    let open = body[expr_start..]
        .find('{')
        .map(|value| expr_start + value)
        .ok_or_else(|| {
            CompileError::Syntax(format!("{handler_kind} `{handler_name}` match missing {{"))
        })?;
    let expr_text = body[expr_start..open].trim();
    if expr_text.is_empty() {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` match expression expected"
        )));
    }
    let expr = parse_expr_in_namespace(expr_text, namespace, program)?;
    validate_expr(&expr, known, program)?;
    let static_type = infer_static_expr_type(&expr, known, program)?;
    let enum_id = static_type
        .scalar()
        .and_then(|scalar| match scalar.value_type {
            ValueType::Enum(id) => Some(id),
            _ => None,
        })
        .ok_or_else(|| {
            CompileError::security(
                "SEC-A10-020",
                format!("{handler_kind} `{handler_name}` match requires a first-class sum type"),
                Some("match an enum/sum value so the compiler can prove exhaustiveness".into()),
            )
        })?;
    let (sum_name, variants): (String, Vec<String>) =
        if enum_id == language_core::TRANSACTION_OUTCOME_ENUM_ID {
            (
                "TransactionOutcome".into(),
                language_core::TRANSACTION_OUTCOME_VARIANTS
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
            )
        } else {
            let definition = program
                .enum_by_id(enum_id)
                .ok_or_else(|| CompileError::Syntax("match references unknown sum type".into()))?;
            (definition.name.clone(), definition.variants.clone())
        };
    let close = matching_brace(body, open).ok_or_else(|| {
        CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` match block unclosed"
        ))
    })?;
    let mut arms = Vec::new();
    let mut arm_cursor = open + 1;
    while arm_cursor < close {
        arm_cursor = skip_ws_and_comments(body, arm_cursor);
        if arm_cursor >= close {
            break;
        }
        let variant = read_ident(body, arm_cursor).ok_or_else(|| {
            CompileError::Syntax(format!(
                "{handler_kind} `{handler_name}` match variant expected"
            ))
        })?;
        if !variants.iter().any(|candidate| candidate == &variant) {
            return Err(CompileError::security(
                "SEC-A10-021",
                format!("sum type `{}` has no variant `{variant}`", sum_name),
                Some("use exactly one declared variant in each match arm".into()),
            ));
        }
        if arms
            .iter()
            .any(|arm: &ParsedMatchArm| arm.variant == variant)
        {
            return Err(CompileError::security(
                "SEC-A10-022",
                format!("match contains duplicate variant `{variant}`"),
                Some("each sum-type variant must appear exactly once".into()),
            ));
        }
        let mut next = skip_ws_and_comments(body, arm_cursor + variant.len());
        if !body[next..close].starts_with("=>") {
            return Err(CompileError::Syntax(format!(
                "match arm `{variant}` requires `=>`"
            )));
        }
        next = skip_ws_and_comments(body, next + 2);
        if body.as_bytes().get(next) != Some(&b'{') {
            return Err(CompileError::Syntax(format!(
                "match arm `{variant}` requires a block"
            )));
        }
        let arm_close = matching_brace(body, next)
            .ok_or_else(|| CompileError::Syntax(format!("match arm `{variant}` block unclosed")))?;
        if arm_close > close {
            return Err(CompileError::Syntax(format!(
                "match arm `{variant}` escapes enclosing match"
            )));
        }
        arms.push(ParsedMatchArm {
            variant,
            body: body[next + 1..arm_close].to_string(),
            body_offset: next + 1,
        });
        arm_cursor = arm_close + 1;
    }
    let missing: Vec<_> = variants
        .iter()
        .filter(|variant| !arms.iter().any(|arm| &arm.variant == *variant))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(CompileError::security(
            "SEC-A10-023",
            format!("non-exhaustive match for `{}`; missing {}", sum_name, missing.join(", ")),
            Some("handle every declared variant; wildcard arms are intentionally not supported for security-critical states".into()),
        ));
    }
    Ok(ParsedMatch {
        expr,
        enum_id,
        arms,
        close,
    })
}
