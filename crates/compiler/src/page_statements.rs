use crate::authorization::parse_and_refine_authorization;
use crate::diagnostics::CompileError;
use crate::domain_refinement;
use crate::domain_symbols::display_domain_symbol;
use crate::expression::{infer_static_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::StaticType;
use crate::input_security::handler_input_types;
use crate::public_errors::parse_fail_statement;
use crate::source_syntax::{
    find_statement_end, is_identifier, matching_brace, preview, read_ident, skip_ws_and_comments,
};
use crate::statement_helpers::parse_query_call;
use crate::{arrays, control_flow, dicts, sum_match};
use language_core::{
    FunctionParam, PageMatchArm, Program, QueryCapability, ResourceUse, SourceLocation, Statement,
    ValueType,
};

pub(super) fn parse_page_statements(
    name: &str,
    namespace: &str,
    body: &str,
    params: &[FunctionParam],
    p: &mut Program,
    source_name: &str,
    base_line: usize,
    allow_resource: bool,
) -> Result<Vec<Statement>, CompileError> {
    let mut known = handler_input_types(name, params, p);
    known.insert(
        "csrfToken".into(),
        StaticType::trusted_scalar(ValueType::String),
    );
    known.insert(
        "authPrincipal".into(),
        StaticType::trusted_scalar(ValueType::String),
    );
    known.insert(
        "authMfaVerified".into(),
        StaticType::trusted_scalar(ValueType::Bool),
    );
    parse_page_statements_known(
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

fn parse_page_statements_known(
    name: &str,
    namespace: &str,
    body: &str,
    p: &mut Program,
    source_name: &str,
    base_line: usize,
    allow_resource: bool,
    known: &mut std::collections::HashMap<String, StaticType>,
) -> Result<Vec<Statement>, CompileError> {
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
                    "page `{name}` nested resource profiles are not supported in v0.1"
                )));
            }
            let profile_start = cursor + "with resource ".len();
            let profile = read_ident(body, profile_start).ok_or_else(|| {
                CompileError::Syntax(format!("page `{name}` resource profile name expected"))
            })?;
            let after = profile_start + profile.len();
            let open = body[after..].find('{').map(|v| after + v).ok_or_else(|| {
                CompileError::Syntax(format!("page `{name}` resource block missing {{"))
            })?;
            if !body[after..open].trim().is_empty() {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` invalid resource profile syntax"
                )));
            }
            let close = matching_brace(body, open).ok_or_else(|| {
                CompileError::Syntax(format!("page `{name}` resource block unclosed"))
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
            let inner = parse_page_statements_known(
                name,
                namespace,
                &body[open + 1..close],
                p,
                source_name,
                inner_base,
                false,
                &mut inner_known,
            )?;
            out.push(Statement::Resource {
                profile,
                source,
                statements: inner,
            });
            cursor = skip_ws_and_comments(body, close + 1);
            if cursor < body.len() {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` resource block must be the final statement in v0.1"
                )));
            }
            continue;
        }
        if let Some(after) = crate::rust_surface_syntax::let_binding_start(body, cursor) {
            let eq = body[after..]
                .find('=')
                .map(|v| after + v)
                .ok_or_else(|| CompileError::Syntax(format!("page `{name}` let has no =")))?;
            let local = body[after..eq].trim();
            if !is_identifier(local)
                || matches!(
                    local,
                    "csrfToken"
                        | "authPrincipal"
                        | "authMfaVerified"
                        | "__flashKind"
                        | "__flashMessage"
                )
            {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` invalid local `{local}`"
                )));
            }
            let end = find_statement_end(body, eq + 1)?;
            let rhs = body[eq + 1..end].trim();
            if let Some((function, args, return_type)) =
                crate::pure_calls::parse_value_call(rhs, namespace, &known, p)?
            {
                known.insert(local.into(), return_type);
                out.push(Statement::PureCall {
                    target: Some(local.into()),
                    function,
                    args,
                });
            } else if let Some(refinement) = domain_refinement::parse(rhs, namespace, &known, p)? {
                known.insert(local.into(), refinement.static_type);
                out.push(Statement::LetValidated {
                    name: local.into(),
                    domain: refinement.domain,
                    expr: refinement.expr,
                });
            } else if let Some((call, ty)) =
                crate::outbound_calls::parse(rhs, namespace, p, &known, false)?
            {
                known.insert(local.into(), ty);
                out.push(Statement::LetOutboundStatus {
                    name: local.into(),
                    call,
                });
            } else if let Some((call, ty)) =
                parse_query_call(rhs, namespace, p, &known, QueryCapability::Db)?
            {
                known.insert(local.into(), ty);
                out.push(Statement::LetQuery {
                    name: local.into(),
                    call,
                });
            } else {
                let expr = parse_expr_in_namespace(rhs, namespace, p)?;
                validate_expr(&expr, &known, p)?;
                crate::visibility::validate_pure_expr_access(&expr, &known, p, namespace)?;
                known.insert(local.into(), infer_static_expr_type(&expr, &known, p)?);
                out.push(Statement::Let {
                    name: local.into(),
                    expr,
                });
            }
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("match ") {
            let parsed = sum_match::parse_match("page", name, namespace, body, cursor, known, p)?;
            let mut arms = Vec::new();
            for arm in parsed.arms {
                let mut arm_known = known.clone();
                let arm_base = base_line
                    + body[..arm.body_offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count();
                let statements = parse_page_statements_known(
                    name,
                    namespace,
                    &arm.body,
                    p,
                    source_name,
                    arm_base,
                    false,
                    &mut arm_known,
                )?;
                arms.push(PageMatchArm {
                    variant: arm.variant,
                    statements,
                });
            }
            out.push(Statement::Match {
                expr: parsed.expr,
                enum_id: parsed.enum_id,
                arms,
            });
            cursor = parsed.close + 1;
            continue;
        }
        if body[cursor..].starts_with("while ") {
            let (condition, statements, close) =
                control_flow::parse_while_block("page", name, namespace, body, cursor, &known, p)?;
            out.push(Statement::While {
                condition,
                statements,
            });
            cursor = close + 1;
            continue;
        }
        if body[cursor..].starts_with("if ") {
            let (condition, statements, close) =
                control_flow::parse_if_block("page", name, namespace, body, cursor, &known, p)?;
            out.push(Statement::If {
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
                        let (array, index, value) =
                            arrays::parse_f32_array_set("page", name, &text, namespace, &known, p)?;
                        out.push(Statement::F32ArraySet {
                            array,
                            index,
                            value,
                        });
                    }
                    Some(value) if value.is_scalar(ValueType::StringDict) => {
                        let (dict, key, value) = dicts::parse_string_dict_set(
                            "page", name, &text, namespace, &known, p,
                        )?;
                        out.push(Statement::StringDictSet { dict, key, value });
                    }
                    _ => {
                        return Err(CompileError::Syntax(format!(
                            "page `{name}` set target `{collection}` is not a mutable collection"
                        )));
                    }
                }
            } else {
                let (target, rhs) = text.split_once('=').ok_or_else(|| {
                    CompileError::Syntax(format!("page `{name}` assignment requires ="))
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
                        "page `{name}` set `{target}` type mismatch"
                    )));
                }
                out.push(Statement::Set {
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
                parse_and_refine_authorization(text, &mut *known, p, &format!("page `{name}`"))?;
            out.push(Statement::Authorize(rule));
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("canonical slug ") {
            let (statement, end) = crate::page_canonical::parse(
                name,
                namespace,
                body,
                cursor,
                allow_resource,
                &out,
                known,
                p,
            )?;
            out.push(statement);
            cursor = end;
            continue;
        }
        if let Some((function, args, next)) =
            crate::pure_calls::parse(body, cursor, namespace, known, p)?
        {
            out.push(Statement::PureCall {
                target: None,
                function,
                args,
            });
            cursor = next;
            continue;
        }
        if let Some((statement, next)) =
            crate::page_response_tail::parse(name, namespace, body, cursor, &known, p)?
        {
            out.push(statement);
            cursor = next;
            continue;
        }
        if let Some((error, next)) = parse_fail_statement("page", name, body, cursor)? {
            out.push(Statement::Fail(error));
            cursor = next;
            continue;
        }
        return Err(CompileError::Syntax(format!(
            "page `{name}` unsupported statement near `{}`",
            preview(&body[cursor..])
        )));
    }
    if !matches!(
        out.last(),
        Some(Statement::ReturnHtml(_))
            | Some(Statement::ReturnJson(_))
            | Some(Statement::ReturnJsonProjection(_))
            | Some(Statement::ReturnTypedJson { .. })
            | Some(Statement::Fail(_))
            | Some(Statement::Resource { .. })
            | Some(Statement::Match { .. })
    ) {
        return Err(CompileError::Syntax(format!(
            "page `{name}` must return Html, Json, or fail with a public error"
        )));
    }
    Ok(out)
}
