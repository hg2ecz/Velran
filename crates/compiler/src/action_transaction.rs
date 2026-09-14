use crate::diagnostics::CompileError;
use crate::handler_types::StaticType;
use crate::source_syntax::{
    find_statement_end, is_identifier, matching_brace, skip_ws_and_comments,
};
use crate::statement_helpers::{parse_business_audit, parse_query_call, parse_security_event};
use language_core::{ActionStatement, Program, QueryCapability, QueryReturn, TxStatement};
use std::collections::HashMap;

pub(super) fn parse(
    action: &str,
    namespace: &str,
    body: &str,
    cursor: usize,
    known: &mut HashMap<String, StaticType>,
    program: &mut Program,
) -> Result<(ActionStatement, usize), CompileError> {
    let open = body[cursor + "transaction db".len()..]
        .find('{')
        .map(|offset| cursor + "transaction db".len() + offset)
        .ok_or_else(|| CompileError::Syntax(format!("action `{action}` transaction missing {{")))?;
    let close = matching_brace(body, open)
        .ok_or_else(|| CompileError::Syntax(format!("action `{action}` transaction unclosed")))?;
    let statements = parse_body(action, namespace, &body[open + 1..close], known, program)?;
    if statements.is_empty() {
        return Err(CompileError::Syntax(format!(
            "action `{action}` empty transaction"
        )));
    }
    Ok((
        ActionStatement::Transaction {
            outcome: None,
            statements,
        },
        close,
    ))
}

fn parse_body(
    action: &str,
    namespace: &str,
    body: &str,
    known: &mut HashMap<String, StaticType>,
    program: &mut Program,
) -> Result<Vec<TxStatement>, CompileError> {
    let mut tx_known = known.clone();
    let mut statements = Vec::new();
    let mut cursor = 0;
    while cursor < body.len() {
        cursor = skip_ws_and_comments(body, cursor);
        if cursor >= body.len() {
            break;
        }
        let end = find_statement_end(body, cursor)?;
        let line = body[cursor..end].trim().trim_end_matches(';').trim();
        statements.push(parse_line(
            action,
            namespace,
            line,
            known,
            &mut tx_known,
            program,
        )?);
        cursor = end + 1;
    }
    Ok(statements)
}

fn parse_line(
    action: &str,
    namespace: &str,
    line: &str,
    known: &mut HashMap<String, StaticType>,
    tx_known: &mut HashMap<String, StaticType>,
    program: &mut Program,
) -> Result<TxStatement, CompileError> {
    if line.starts_with("audit ") {
        return Ok(TxStatement::BusinessAudit(parse_business_audit(
            action, namespace, line, tx_known, program,
        )?));
    }
    if line.starts_with("security ") {
        return Ok(TxStatement::BusinessAudit(parse_security_event(
            action, namespace, line, tx_known, program,
        )?));
    }
    if let Some(rest) = line.strip_prefix("let ") {
        return parse_let(action, namespace, line, rest, known, tx_known, program);
    }
    let (call, _) = parse_query_call(
        line,
        namespace,
        program,
        tx_known,
        QueryCapability::Transaction,
    )?
    .ok_or_else(|| {
        CompileError::Syntax(format!(
            "transaction only supports mutating query calls; got `{line}`"
        ))
    })?;
    Ok(TxStatement::Query(call))
}

fn parse_let(
    _action: &str,
    namespace: &str,
    line: &str,
    rest: &str,
    known: &mut HashMap<String, StaticType>,
    tx_known: &mut HashMap<String, StaticType>,
    program: &mut Program,
) -> Result<TxStatement, CompileError> {
    let (local, rhs) = rest
        .split_once('=')
        .ok_or_else(|| CompileError::Syntax(format!("transaction let `{line}` missing =")))?;
    let local = local.trim();
    if !is_identifier(local) {
        return Err(CompileError::Syntax(format!(
            "invalid transaction local `{local}`"
        )));
    }
    let (call, ty) = parse_query_call(
        rhs.trim(),
        namespace,
        program,
        tx_known,
        QueryCapability::Transaction,
    )?
    .ok_or_else(|| {
        CompileError::Syntax(format!(
            "transaction let requires a mutating query call; got `{line}`"
        ))
    })?;
    if matches!(
        program.query(&call.query).map(|query| &query.return_type),
        Some(&QueryReturn::Void)
    ) {
        return Err(CompileError::Syntax(format!(
            "void query `{}` cannot be assigned to `{local}`",
            call.query
        )));
    }
    tx_known.insert(local.into(), ty.clone());
    known.insert(local.into(), ty);
    Ok(TxStatement::LetQuery {
        name: local.into(),
        call,
    })
}
