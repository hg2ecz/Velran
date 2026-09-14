use crate::diagnostics::CompileError;
use crate::source_syntax::{find_statement_end, is_identifier};
use language_core::PublicError;

pub(super) fn parse_fail_statement(
    kind: &str,
    function: &str,
    body: &str,
    cursor: usize,
) -> Result<Option<(PublicError, usize)>, CompileError> {
    if !body[cursor..].starts_with("fail ") {
        return Ok(None);
    }
    let end = find_statement_end(body, cursor)?;
    let raw = body[cursor + "fail ".len()..end].trim();
    if !is_identifier(raw) {
        return Err(syntax(kind, function));
    }
    let error = PublicError::from_source_name(raw).ok_or_else(|| {
        CompileError::security(
            "SEC-A10-003",
            format!(
                "{kind} `{function}` cannot expose unknown or internal error `{raw}`; use one of badRequest, notFound, forbidden, conflict"
            ),
            Some(
                "replace the error with a supported public error and keep internal details platform-owned"
                    .into(),
            ),
        )
    })?;
    Ok(Some((error, end + 1)))
}

fn syntax(kind: &str, function: &str) -> CompileError {
    CompileError::Syntax(format!(
        "{kind} `{function}` fail syntax is `fail badRequest|notFound|forbidden|conflict;`"
    ))
}
