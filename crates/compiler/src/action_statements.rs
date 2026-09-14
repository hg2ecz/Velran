use crate::authorization::parse_and_refine_authorization;
use crate::diagnostics::CompileError;
use crate::domain_refinement;
use crate::domain_symbols::display_domain_symbol;
use crate::expression::{infer_static_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::{HandlerReturnKind, StaticType};
use crate::input_security::handler_input_types;
use crate::public_errors::parse_fail_statement;
use crate::source_syntax::{
    find_statement_end, is_identifier, matching_brace, preview, read_ident, skip_ws_and_comments,
};
use crate::statement_helpers::{parse_business_audit, parse_query_call, parse_security_event};
use crate::{arrays, control_flow, dicts, sum_match};
use language_core::{
    ActionMatchArm, ActionStatement, FunctionParam, Program, QueryCapability, ResourceUse,
    SourceLocation, TxStatement, ValueType,
};

fn flash_count(items: &[ActionStatement]) -> usize {
    items
        .iter()
        .map(|statement| match statement {
            ActionStatement::Flash(_) => 1,
            ActionStatement::Resource { statements, .. } => flash_count(statements),
            ActionStatement::Match { arms, .. } => {
                arms.iter().map(|arm| flash_count(&arm.statements)).sum()
            }
            _ => 0,
        })
        .sum()
}

fn validate_flash_policy(name: &str, statements: &[ActionStatement]) -> Result<(), CompileError> {
    let flashes = flash_count(statements);
    if flashes > 1 {
        return Err(CompileError::Syntax(format!(
            "action `{name}` may set at most one flash message"
        )));
    }
    if flashes == 1 && !control_flow::action_return_matches(statements, HandlerReturnKind::Redirect)
    {
        return Err(CompileError::Syntax(format!(
            "action `{name}` flash requires a Redirect return"
        )));
    }
    Ok(())
}

pub(super) fn parse_action_statements(
    name: &str,
    namespace: &str,
    body: &str,
    params: &[FunctionParam],
    p: &mut Program,
    source_name: &str,
    base_line: usize,
    allow_resource: bool,
) -> Result<Vec<ActionStatement>, CompileError> {
    let mut known = handler_input_types(name, params, p);
    known.insert(
        "authPrincipal".into(),
        StaticType::trusted_scalar(ValueType::String),
    );
    known.insert(
        "authMfaVerified".into(),
        StaticType::trusted_scalar(ValueType::Bool),
    );
    parse_action_statements_known(
        name,
        namespace,
        body,
        p,
        source_name,
        base_line,
        allow_resource,
        &mut known,
    )
}
fn parse_action_statements_known(
    name: &str,
    namespace: &str,
    body: &str,
    p: &mut Program,
    source_name: &str,
    base_line: usize,
    allow_resource: bool,
    known: &mut std::collections::HashMap<String, StaticType>,
) -> Result<Vec<ActionStatement>, CompileError> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while cursor < body.len() {
        cursor = skip_ws_and_comments(body, cursor);
        if cursor >= body.len() {
            break;
        }
        if body[cursor..].starts_with("with resource ") {
            if !allow_resource {
                return Err(CompileError::Syntax(format!(
                    "action `{name}` nested resource profiles are not supported in v0.1"
                )));
            }
            let profile_start = cursor + "with resource ".len();
            let profile = read_ident(body, profile_start).ok_or_else(|| {
                CompileError::Syntax(format!("action `{name}` resource profile name expected"))
            })?;
            let after = profile_start + profile.len();
            let open = body[after..].find('{').map(|v| after + v).ok_or_else(|| {
                CompileError::Syntax(format!("action `{name}` resource block missing {{"))
            })?;
            if !body[after..open].trim().is_empty() {
                return Err(CompileError::Syntax(format!(
                    "action `{name}` invalid resource profile syntax"
                )));
            }
            let close = matching_brace(body, open).ok_or_else(|| {
                CompileError::Syntax(format!("action `{name}` resource block unclosed"))
            })?;
            let line = base_line + body[..cursor].bytes().filter(|b| *b == b'\n').count();
            let source = SourceLocation {
                file: source_name.into(),
                line,
                function: display_domain_symbol(name),
            };
            p.resource_uses.push(ResourceUse {
                profile: profile.clone(),
                source: source.clone(),
            });
            let inner_base = base_line + body[..open + 1].bytes().filter(|b| *b == b'\n').count();
            let mut inner_known = known.clone();
            let inner = parse_action_statements_known(
                name,
                namespace,
                &body[open + 1..close],
                p,
                source_name,
                inner_base,
                false,
                &mut inner_known,
            )?;
            out.push(ActionStatement::Resource {
                profile,
                source,
                statements: inner,
            });
            cursor = skip_ws_and_comments(body, close + 1);
            if cursor < body.len() {
                return Err(CompileError::Syntax(format!(
                    "action `{name}` resource block must be the final statement in v0.1"
                )));
            }
            continue;
        }
        if body[cursor..].starts_with("transaction db") {
            let (statement, close) =
                crate::action_transaction::parse(name, namespace, body, cursor, known, p)?;
            out.push(statement);
            cursor = close + 1;
            continue;
        }
        if let Some(after_let) = crate::rust_surface_syntax::let_binding_start(body, cursor) {
            if let Some(eq_rel) = body[after_let..].find('=') {
                let eq = after_let + eq_rel;
                let local = body[after_let..eq].trim();
                let rhs_start = skip_ws_and_comments(body, eq + 1);
                if is_identifier(local) && body[rhs_start..].starts_with("transaction db") {
                    let open = body[rhs_start + "transaction db".len()..]
                        .find('{')
                        .map(|v| rhs_start + "transaction db".len() + v)
                        .ok_or_else(|| {
                            CompileError::Syntax(format!("action `{name}` transaction missing {{"))
                        })?;
                    let close = matching_brace(body, open).ok_or_else(|| {
                        CompileError::Syntax(format!("action `{name}` transaction unclosed"))
                    })?;
                    let mut tx_known = known.clone();
                    let mut statements = Vec::new();
                    let mut tx_cursor = 0;
                    let tx_body = &body[open + 1..close];
                    while tx_cursor < tx_body.len() {
                        tx_cursor = skip_ws_and_comments(tx_body, tx_cursor);
                        if tx_cursor >= tx_body.len() {
                            break;
                        }
                        let end = find_statement_end(tx_body, tx_cursor)?;
                        let line = tx_body[tx_cursor..end].trim().trim_end_matches(';').trim();
                        if line.starts_with("audit ") {
                            statements.push(TxStatement::BusinessAudit(parse_business_audit(
                                name, namespace, line, &tx_known, p,
                            )?));
                        } else if line.starts_with("security ") {
                            statements.push(TxStatement::BusinessAudit(parse_security_event(
                                name, namespace, line, &tx_known, p,
                            )?));
                        } else if line.starts_with("let ") {
                            let rest = line
                                .strip_prefix("let mut ")
                                .or_else(|| line.strip_prefix("let "))
                                .unwrap();
                            let (tx_local, rhs) = rest.split_once('=').ok_or_else(|| {
                                CompileError::Syntax(format!("transaction let `{line}` missing ="))
                            })?;
                            let tx_local = tx_local.trim();
                            let (call, ty) = parse_query_call(
                                rhs.trim(),
                                namespace,
                                p,
                                &tx_known,
                                QueryCapability::Transaction,
                            )?
                            .ok_or_else(|| {
                                CompileError::Syntax(format!(
                                    "transaction let requires a mutating query call; got `{line}`"
                                ))
                            })?;
                            tx_known.insert(tx_local.into(), ty.clone());
                            known.insert(tx_local.into(), ty);
                            statements.push(TxStatement::LetQuery {
                                name: tx_local.into(),
                                call,
                            });
                        } else {
                            let (call, _) = parse_query_call(
                                line,
                                namespace,
                                p,
                                &tx_known,
                                QueryCapability::Transaction,
                            )?
                            .ok_or_else(|| {
                                CompileError::Syntax(format!(
                                    "transaction only supports mutating query calls; got `{line}`"
                                ))
                            })?;
                            statements.push(TxStatement::Query(call));
                        }
                        tx_cursor = end + 1;
                    }
                    if statements.is_empty() {
                        return Err(CompileError::Syntax(format!(
                            "action `{name}` empty transaction"
                        )));
                    }
                    known.insert(
                        local.into(),
                        StaticType::trusted_scalar(ValueType::Enum(
                            language_core::TRANSACTION_OUTCOME_ENUM_ID,
                        )),
                    );
                    out.push(ActionStatement::Transaction {
                        outcome: Some(local.into()),
                        statements,
                    });
                    cursor = skip_ws_and_comments(body, close + 1);
                    if body.as_bytes().get(cursor) == Some(&b';') {
                        cursor += 1;
                    }
                    continue;
                }
            }
            let after = cursor + 4;
            let eq = body[after..]
                .find('=')
                .map(|v| after + v)
                .ok_or_else(|| CompileError::Syntax(format!("action `{name}` let has no =")))?;
            let local = body[after..eq].trim();
            if !is_identifier(local)
                || matches!(
                    local,
                    "authPrincipal" | "authMfaVerified" | "__flashKind" | "__flashMessage"
                )
            {
                return Err(CompileError::Syntax(format!(
                    "action `{name}` invalid local `{local}`"
                )));
            }
            let end = find_statement_end(body, eq + 1)?;
            let rhs = body[eq + 1..end].trim();
            if let Some(refinement) = domain_refinement::parse(rhs, namespace, &known, p)? {
                known.insert(local.into(), refinement.static_type);
                out.push(ActionStatement::LetValidated {
                    name: local.into(),
                    domain: refinement.domain,
                    expr: refinement.expr,
                });
            } else if let Some((call, ty)) =
                crate::outbound_calls::parse(rhs, namespace, p, &known, true)?
            {
                known.insert(local.into(), ty);
                out.push(ActionStatement::LetOutboundStatus {
                    name: local.into(),
                    call,
                });
            } else if let Some((call, ty)) =
                parse_query_call(rhs, namespace, p, &known, QueryCapability::Db)?
            {
                known.insert(local.into(), ty);
                out.push(ActionStatement::LetQuery {
                    name: local.into(),
                    call,
                });
            } else {
                let expr = parse_expr_in_namespace(rhs, namespace, p)?;
                validate_expr(&expr, &known, p)?;
                crate::visibility::validate_pure_expr_access(&expr, &known, p, namespace)?;
                known.insert(local.into(), infer_static_expr_type(&expr, &known, p)?);
                out.push(ActionStatement::Let {
                    name: local.into(),
                    expr,
                });
            }
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("match ") {
            let parsed = sum_match::parse_match("action", name, namespace, body, cursor, known, p)?;
            let mut arms = Vec::new();
            for arm in parsed.arms {
                let mut arm_known = known.clone();
                let arm_base = base_line
                    + body[..arm.body_offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count();
                let statements = parse_action_statements_known(
                    name,
                    namespace,
                    &arm.body,
                    p,
                    source_name,
                    arm_base,
                    false,
                    &mut arm_known,
                )?;
                arms.push(ActionMatchArm {
                    variant: arm.variant,
                    statements,
                });
            }
            out.push(ActionStatement::Match {
                expr: parsed.expr,
                enum_id: parsed.enum_id,
                arms,
            });
            cursor = parsed.close + 1;
            continue;
        }
        if body[cursor..].starts_with("while ") {
            let (condition, statements, close) = control_flow::parse_while_block(
                "action", name, namespace, body, cursor, &known, p,
            )?;
            out.push(ActionStatement::While {
                condition,
                statements,
            });
            cursor = close + 1;
            continue;
        }
        if body[cursor..].starts_with("if ") {
            let (condition, statements, close) =
                control_flow::parse_if_block("action", name, namespace, body, cursor, &known, p)?;
            out.push(ActionStatement::If {
                condition,
                statements,
            });
            cursor = close + 1;
            continue;
        }
        if crate::rust_surface_syntax::direct_assignment_start(body, cursor) {
            let end = find_statement_end(body, cursor)?;
            let text = crate::rust_surface_syntax::assignment_text(body, cursor, end);
            if text
                .split_once('=')
                .map(|(lhs, _)| lhs.contains('['))
                .unwrap_or(false)
            {
                let target = text
                    .split_once('=')
                    .map(|(lhs, _)| lhs.trim())
                    .unwrap_or("");
                let collection = target.split('[').next().unwrap_or("").trim();
                match known.get(collection) {
                    Some(value) if value.is_scalar(ValueType::F32Array) => {
                        let (array, index, value) = arrays::parse_f32_array_set(
                            "action", name, &text, namespace, &known, p,
                        )?;
                        out.push(ActionStatement::F32ArraySet {
                            array,
                            index,
                            value,
                        });
                    }
                    Some(value) if value.is_scalar(ValueType::StringDict) => {
                        let (dict, key, value) = dicts::parse_string_dict_set(
                            "action", name, &text, namespace, &known, p,
                        )?;
                        out.push(ActionStatement::StringDictSet { dict, key, value });
                    }
                    _ => {
                        return Err(CompileError::Syntax(format!(
                            "action `{name}` set target `{collection}` is not a mutable collection"
                        )));
                    }
                }
            } else {
                let (target, rhs) = text.split_once('=').ok_or_else(|| {
                    CompileError::Syntax(format!("action `{name}` assignment requires ="))
                })?;
                let target = target.trim();
                let expected = known
                    .get(target)
                    .cloned()
                    .ok_or_else(|| CompileError::UnknownVariable(target.into()))?;
                let expr = parse_expr_in_namespace(rhs.trim(), namespace, p)?;
                validate_expr(&expr, &known, p)?;
                crate::visibility::validate_pure_expr_access(&expr, &known, p, namespace)?;
                if infer_static_expr_type(&expr, &known, p)? != expected {
                    return Err(CompileError::Syntax(format!(
                        "action `{name}` set `{target}` type mismatch"
                    )));
                }
                out.push(ActionStatement::Set {
                    name: target.into(),
                    expr,
                });
            }
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("authorize ") {
            let end = find_statement_end(body, cursor)?;
            let text = body[cursor..end].trim().trim_end_matches(';').trim();
            let rule =
                parse_and_refine_authorization(text, &mut *known, p, &format!("action `{name}`"))?;
            out.push(ActionStatement::Authorize(rule));
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("flash ") {
            let (statement, end) = crate::action_flash::parse(name, namespace, body, cursor, p)?;
            out.push(statement);
            cursor = end;
            continue;
        }
        if let Some((statement, next)) =
            crate::action_response_tail::parse(name, namespace, body, cursor, &known, p)?
        {
            out.push(statement);
            cursor = next;
            continue;
        }
        if let Some((error, next)) = parse_fail_statement("action", name, body, cursor)? {
            out.push(ActionStatement::Fail(error));
            cursor = next;
            continue;
        }
        return Err(CompileError::Syntax(format!(
            "action `{name}` unsupported statement near `{}`",
            preview(&body[cursor..])
        )));
    }
    if !matches!(
        out.last(),
        Some(ActionStatement::ReturnRedirect(_))
            | Some(ActionStatement::ReturnJson(_))
            | Some(ActionStatement::ReturnJsonProjection(_))
            | Some(ActionStatement::ReturnTypedJson { .. })
            | Some(ActionStatement::Fail(_))
            | Some(ActionStatement::Resource { .. })
            | Some(ActionStatement::Match { .. })
    ) {
        return Err(CompileError::Syntax(format!(
            "action `{name}` must return Redirect, Json, or fail with a public error"
        )));
    }
    validate_flash_policy(name, &out)?;
    Ok(out)
}
