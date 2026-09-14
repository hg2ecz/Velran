use crate::diagnostics::CompileError;
use crate::expression::parse_expr_in_namespace;
use crate::source_syntax::find_statement_end;
use language_core::{ActionStatement, Expr, FlashKind, FlashMessage, Program};

pub(super) fn parse(
    action: &str,
    namespace: &str,
    body: &str,
    cursor: usize,
    program: &Program,
) -> Result<(ActionStatement, usize), CompileError> {
    let end = find_statement_end(body, cursor)?;
    let text = body[cursor..end].trim().trim_end_matches(';').trim();
    let rest = text.strip_prefix("flash ").unwrap_or_default().trim();
    let (kind_raw, message_raw) = rest.split_once(char::is_whitespace).ok_or_else(|| {
        CompileError::Syntax(format!(
            "action `{action}` flash requires kind and string literal"
        ))
    })?;
    let kind = parse_kind(action, kind_raw)?;
    let expr = parse_expr_in_namespace(message_raw.trim(), namespace, program)?;
    let Expr::String(message) = expr else {
        return Err(CompileError::Syntax(format!(
            "action `{action}` flash message must be a compiler-owned string literal"
        )));
    };
    validate_message(action, &message)?;
    Ok((
        ActionStatement::Flash(FlashMessage { kind, message }),
        end + 1,
    ))
}

fn parse_kind(action: &str, raw: &str) -> Result<FlashKind, CompileError> {
    match raw {
        "success" => Ok(FlashKind::Success),
        "info" => Ok(FlashKind::Info),
        "warning" => Ok(FlashKind::Warning),
        "error" => Ok(FlashKind::Error),
        _ => Err(CompileError::Syntax(format!(
            "action `{action}` flash kind must be success, info, warning, or error"
        ))),
    }
}

fn validate_message(action: &str, message: &str) -> Result<(), CompileError> {
    if message.is_empty()
        || message.len() > 200
        || message
            .bytes()
            .any(|byte| matches!(byte, b'\r' | b'\n' | 0))
    {
        return Err(CompileError::Syntax(format!(
            "action `{action}` flash message must be 1..200 bytes and single-line"
        )));
    }
    Ok(())
}
