use crate::cache_safety::expr_uses_request_state;
use crate::diagnostics::CompileError;
#[cfg(test)]
use crate::expression::parse_expr;
use crate::expression::{
    infer_expr_type, infer_static_expr_type, parse_expr_in_namespace, validate_expr,
};
use crate::handler_types::{HandlerReturnKind, StaticType};
use crate::source_syntax::{
    find_statement_end, is_identifier, matching_brace, preview, skip_ws_and_comments,
};
use crate::{arrays, dicts};
use language_core::{ActionStatement, ComputeStatement, Expr, Program, Statement, ValueType};
use std::collections::HashMap;

fn fresh_internal_local(prefix: &str, seed: usize, known: &HashMap<String, StaticType>) -> String {
    let mut index = seed;
    loop {
        let candidate = format!("{prefix}{index}");
        if !known.contains_key(&candidate) {
            return candidate;
        }
        index = index.saturating_add(1);
    }
}

fn validate_compute_expr(
    expr: &language_core::Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
    namespace: &str,
) -> Result<(), CompileError> {
    validate_expr(expr, known, program)?;
    crate::visibility::validate_pure_expr_access(expr, known, program, namespace)
}

fn line_for_offset(body: &str, cursor: usize, line_base: Option<usize>) -> Option<usize> {
    line_base.map(|base| {
        base + body[..cursor.min(body.len())]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
    })
}

fn with_expression_line(err: CompileError, line: Option<usize>) -> CompileError {
    let Some(line) = line else {
        return err;
    };
    match err {
        CompileError::Syntax(message) if !message.starts_with("line ") => {
            CompileError::Syntax(format!("line {line}: {message}"))
        }
        other => other,
    }
}

fn parse_expr_at(
    raw: &str,
    namespace: &str,
    p: &Program,
    body: &str,
    cursor: usize,
    line_base: Option<usize>,
) -> Result<Expr, CompileError> {
    parse_expr_in_namespace(raw, namespace, p)
        .map_err(|err| with_expression_line(err, line_for_offset(body, cursor, line_base)))
}

pub(super) fn parse_while_block(
    handler_kind: &str,
    handler_name: &str,
    namespace: &str,
    body: &str,
    cursor: usize,
    known: &HashMap<String, StaticType>,
    p: &Program,
    line_base: Option<usize>,
) -> Result<(Expr, Vec<ComputeStatement>, usize), CompileError> {
    let cond_start = cursor + "while ".len();
    let open = body[cond_start..]
        .find('{')
        .map(|v| cond_start + v)
        .ok_or_else(|| {
            CompileError::Syntax(format!("{handler_kind} `{handler_name}` while missing {{"))
        })?;
    let condition = parse_expr_at(
        body[cond_start..open].trim(),
        namespace,
        p,
        body,
        cond_start,
        line_base,
    )?;
    validate_compute_expr(&condition, known, p, namespace)?;
    if infer_expr_type(&condition, known, p)? != ValueType::Bool {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` while condition must be Bool"
        )));
    }
    let close = matching_brace(body, open).ok_or_else(|| {
        CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` while block unclosed"
        ))
    })?;
    let mut inner_known = known.clone();
    let statements = parse_compute_statements(
        handler_kind,
        handler_name,
        namespace,
        &body[open + 1..close],
        &mut inner_known,
        p,
        line_base.map(|base| base + body[..open + 1].bytes().filter(|b| *b == b'\n').count()),
    )?;
    Ok((condition, statements, close))
}

pub(super) fn parse_if_block(
    handler_kind: &str,
    handler_name: &str,
    namespace: &str,
    body: &str,
    cursor: usize,
    known: &HashMap<String, StaticType>,
    p: &Program,
    line_base: Option<usize>,
) -> Result<(Expr, Vec<ComputeStatement>, usize), CompileError> {
    let cond_start = cursor + "if ".len();
    let open = body[cond_start..]
        .find('{')
        .map(|v| cond_start + v)
        .ok_or_else(|| {
            CompileError::Syntax(format!("{handler_kind} `{handler_name}` if missing {{"))
        })?;
    let condition = parse_expr_at(
        body[cond_start..open].trim(),
        namespace,
        p,
        body,
        cond_start,
        line_base,
    )?;
    validate_compute_expr(&condition, known, p, namespace)?;
    if infer_expr_type(&condition, known, p)? != ValueType::Bool {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` if condition must be Bool"
        )));
    }
    let close = matching_brace(body, open).ok_or_else(|| {
        CompileError::Syntax(format!("{handler_kind} `{handler_name}` if block unclosed"))
    })?;
    let mut inner_known = known.clone();
    let statements = parse_compute_statements(
        handler_kind,
        handler_name,
        namespace,
        &body[open + 1..close],
        &mut inner_known,
        p,
        line_base.map(|base| base + body[..open + 1].bytes().filter(|b| *b == b'\n').count()),
    )?;
    Ok((condition, statements, close))
}

fn parse_and_lower_if_chain(
    handler_kind: &str,
    handler_name: &str,
    namespace: &str,
    body: &str,
    cursor: usize,
    known: &mut HashMap<String, StaticType>,
    p: &Program,
    local_seed: usize,
    line_base: Option<usize>,
) -> Result<(Vec<ComputeStatement>, usize), CompileError> {
    let (condition, statements, close) = parse_if_block(
        handler_kind,
        handler_name,
        namespace,
        body,
        cursor,
        known,
        p,
        line_base,
    )?;
    let after_if = skip_ws_and_comments(body, close + 1);
    let has_else = body[after_if..].starts_with("else")
        && body
            .as_bytes()
            .get(after_if + "else".len())
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b'{' || *b == b'/');

    if !has_else {
        return Ok((
            vec![ComputeStatement::If {
                condition,
                statements,
            }],
            close,
        ));
    }

    let after_else = skip_ws_and_comments(body, after_if + "else".len());
    let (else_statements, chain_close) = if body[after_else..].starts_with("if ") {
        let mut else_known = known.clone();
        parse_and_lower_if_chain(
            handler_kind,
            handler_name,
            namespace,
            body,
            after_else,
            &mut else_known,
            p,
            local_seed + 1,
            line_base,
        )?
    } else if body.as_bytes().get(after_else) == Some(&b'{') {
        let else_close = matching_brace(body, after_else).ok_or_else(|| {
            CompileError::Syntax(format!(
                "{handler_kind} `{handler_name}` else block unclosed"
            ))
        })?;
        let mut else_known = known.clone();
        let statements = parse_compute_statements(
            handler_kind,
            handler_name,
            namespace,
            &body[after_else + 1..else_close],
            &mut else_known,
            p,
            line_base.map(|base| {
                base + body[..after_else + 1]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count()
            }),
        )?;
        (statements, else_close)
    } else {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` else must use a block or `else if`"
        )));
    };

    let condition_local = fresh_internal_local("__velran_if_condition_", local_seed, known);
    let condition_type = infer_static_expr_type(&condition, known, p)?;
    known.insert(condition_local.clone(), condition_type);
    Ok((
        vec![
            ComputeStatement::Let {
                name: condition_local.clone(),
                expr: condition,
            },
            ComputeStatement::If {
                condition: language_core::Expr::Variable(condition_local.clone()),
                statements,
            },
            ComputeStatement::If {
                condition: language_core::Expr::Not(Box::new(language_core::Expr::Variable(
                    condition_local,
                ))),
                statements: else_statements,
            },
        ],
        chain_close,
    ))
}

pub(super) fn parse_pure_compute_statements(
    function_name: &str,
    namespace: &str,
    body: &str,
    known: &mut HashMap<String, StaticType>,
    p: &Program,
    line_base: usize,
) -> Result<Vec<ComputeStatement>, CompileError> {
    parse_compute_statements(
        "pure fn",
        function_name,
        namespace,
        body,
        known,
        p,
        Some(line_base),
    )
}

fn parse_compute_statements(
    handler_kind: &str,
    handler_name: &str,
    namespace: &str,
    body: &str,
    known: &mut HashMap<String, StaticType>,
    p: &Program,
    line_base: Option<usize>,
) -> Result<Vec<ComputeStatement>, CompileError> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while cursor < body.len() {
        cursor = skip_ws_and_comments(body, cursor);
        if cursor >= body.len() {
            break;
        }
        if let Some((function, args, next)) =
            crate::pure_calls::parse(body, cursor, namespace, known, p)?
        {
            out.push(ComputeStatement::PureCall {
                target: None,
                function,
                args,
            });
            cursor = next;
            continue;
        }
        if handler_kind == "pure fn" && body[cursor..].starts_with("return ") {
            let end = find_statement_end(body, cursor)?;
            let raw = body[cursor + "return ".len()..end].trim();
            if raw == "None" {
                out.push(ComputeStatement::ReturnOption { value: None });
            } else if let Some(inner) = raw.strip_prefix("Some(").and_then(|v| v.strip_suffix(')'))
            {
                let expr = parse_expr_at(inner.trim(), namespace, p, body, cursor, line_base)?;
                validate_compute_expr(&expr, known, p, namespace)?;
                out.push(ComputeStatement::ReturnOption { value: Some(expr) });
            } else if let Some(inner) = raw.strip_prefix("Ok(").and_then(|v| v.strip_suffix(')')) {
                let expr = parse_expr_at(inner.trim(), namespace, p, body, cursor, line_base)?;
                validate_compute_expr(&expr, known, p, namespace)?;
                out.push(ComputeStatement::ReturnResult {
                    is_ok: true,
                    value: expr,
                });
            } else if let Some(inner) = raw.strip_prefix("Err(").and_then(|v| v.strip_suffix(')')) {
                let expr = parse_expr_at(inner.trim(), namespace, p, body, cursor, line_base)?;
                validate_compute_expr(&expr, known, p, namespace)?;
                out.push(ComputeStatement::ReturnResult {
                    is_ok: false,
                    value: expr,
                });
            } else if let Some((schema, fields)) =
                crate::pure_struct_return::parse(raw, namespace, known, p)?
            {
                out.push(ComputeStatement::ReturnStruct { schema, fields });
            } else if let Some((function, args, return_type)) =
                crate::pure_calls::parse_value_call(raw, namespace, known, p)?
            {
                let target = fresh_internal_local("__velran_return_call_", out.len(), known);
                known.insert(target.clone(), return_type);
                out.push(ComputeStatement::PureCall {
                    target: Some(target.clone()),
                    function,
                    args,
                });
                out.push(ComputeStatement::Return(language_core::Expr::Variable(
                    target,
                )));
            } else {
                let expr = parse_expr_at(raw, namespace, p, body, cursor, line_base)?;
                validate_compute_expr(&expr, known, p, namespace)?;
                out.push(ComputeStatement::Return(expr));
            }
            cursor = end + 1;
            continue;
        }
        if let Some(after) = crate::rust_surface_syntax::let_binding_start(body, cursor) {
            let eq = body[after..].find('=').map(|v| after + v).ok_or_else(|| {
                CompileError::Syntax(format!("{handler_kind} `{handler_name}` let has no ="))
            })?;
            let local = body[after..eq].trim();
            if !is_identifier(local) {
                return Err(CompileError::Syntax(format!(
                    "{handler_kind} `{handler_name}` invalid local `{local}`"
                )));
            }
            let end = find_statement_end(body, eq + 1)?;
            let rhs = body[eq + 1..end].trim();
            if let Some((function, args, return_type)) =
                crate::pure_calls::parse_value_call(rhs, namespace, known, p)?
            {
                known.insert(local.into(), return_type);
                out.push(ComputeStatement::PureCall {
                    target: Some(local.into()),
                    function,
                    args,
                });
                cursor = end + 1;
                continue;
            }
            let expr = parse_expr_at(rhs, namespace, p, body, cursor, line_base)?;
            validate_compute_expr(&expr, known, p, namespace)?;
            known.insert(local.into(), infer_static_expr_type(&expr, known, p)?);
            out.push(ComputeStatement::Let {
                name: local.into(),
                expr,
            });
            cursor = end + 1;
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
                            handler_kind,
                            handler_name,
                            &text,
                            namespace,
                            known,
                            p,
                        )?;
                        out.push(ComputeStatement::F32ArraySet {
                            array,
                            index,
                            value,
                        });
                    }
                    Some(value) if value.is_scalar(ValueType::StringDict) => {
                        let (dict, key, value) = dicts::parse_string_dict_set(
                            handler_kind,
                            handler_name,
                            &text,
                            namespace,
                            known,
                            p,
                        )?;
                        out.push(ComputeStatement::StringDictSet { dict, key, value });
                    }
                    _ => {
                        return Err(CompileError::Syntax(format!(
                            "{handler_kind} `{handler_name}` set target `{collection}` is not a mutable collection"
                        )));
                    }
                }
            } else {
                let (name, rhs) = text.split_once('=').ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "{handler_kind} `{handler_name}` assignment requires ="
                    ))
                })?;
                let name = name.trim();
                if !is_identifier(name) {
                    return Err(CompileError::Syntax(format!(
                        "{handler_kind} `{handler_name}` invalid set target `{name}`"
                    )));
                }
                let expected = known
                    .get(name)
                    .cloned()
                    .ok_or_else(|| CompileError::UnknownVariable(name.into()))?;
                if let Some((function, args, return_type)) =
                    crate::pure_calls::parse_value_call(rhs.trim(), namespace, known, p)?
                {
                    if expected != return_type {
                        return Err(CompileError::Syntax(format!(
                            "{handler_kind} `{handler_name}` set `{name}` type mismatch"
                        )));
                    }
                    let target = fresh_internal_local("__velran_set_call_", out.len(), known);
                    known.insert(target.clone(), return_type);
                    out.push(ComputeStatement::PureCall {
                        target: Some(target.clone()),
                        function,
                        args,
                    });
                    out.push(ComputeStatement::Set {
                        name: name.into(),
                        expr: language_core::Expr::Variable(target),
                    });
                } else {
                    let expr = parse_expr_at(rhs.trim(), namespace, p, body, cursor, line_base)?;
                    validate_compute_expr(&expr, known, p, namespace)?;
                    let actual = infer_static_expr_type(&expr, known, p)?;
                    if expected != actual {
                        return Err(CompileError::Syntax(format!(
                            "{handler_kind} `{handler_name}` set `{name}` type mismatch"
                        )));
                    }
                    out.push(ComputeStatement::Set {
                        name: name.into(),
                        expr,
                    });
                }
            }
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("while ") {
            let (condition, statements, close) = parse_while_block(
                handler_kind,
                handler_name,
                namespace,
                body,
                cursor,
                known,
                p,
                line_base,
            )?;
            out.push(ComputeStatement::While {
                condition,
                statements,
            });
            cursor = close + 1;
            continue;
        }
        if body[cursor..].starts_with("if ") {
            let (lowered, close) = parse_and_lower_if_chain(
                handler_kind,
                handler_name,
                namespace,
                body,
                cursor,
                known,
                p,
                out.len(),
                line_base,
            )?;
            out.extend(lowered);
            cursor = close + 1;
            continue;
        }
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` unsupported compute statement near `{}`",
            preview(&body[cursor..])
        )));
    }
    Ok(out)
}

pub(super) fn page_return_matches(statements: &[Statement], declared: HandlerReturnKind) -> bool {
    match statements.last() {
        Some(Statement::Match { arms, .. }) => arms
            .iter()
            .all(|arm| page_return_matches(&arm.statements, declared)),
        Some(Statement::Resource { statements, .. }) => page_return_matches(statements, declared),
        _ => {
            page_return_kind(statements) == Some(declared) || page_terminates_with_fail(statements)
        }
    }
}

fn page_terminates_with_fail(statements: &[Statement]) -> bool {
    match statements.last() {
        Some(Statement::Fail(_)) => true,
        Some(Statement::Resource { statements, .. }) => page_terminates_with_fail(statements),
        _ => false,
    }
}

pub(super) fn action_return_matches(
    statements: &[ActionStatement],
    declared: HandlerReturnKind,
) -> bool {
    match statements.last() {
        Some(ActionStatement::Match { arms, .. }) => arms
            .iter()
            .all(|arm| action_return_matches(&arm.statements, declared)),
        Some(ActionStatement::Resource { statements, .. }) => {
            action_return_matches(statements, declared)
        }
        _ => {
            action_return_kind(statements) == Some(declared)
                || action_terminates_with_fail(statements)
        }
    }
}

fn action_terminates_with_fail(statements: &[ActionStatement]) -> bool {
    match statements.last() {
        Some(ActionStatement::Fail(_)) => true,
        Some(ActionStatement::Resource { statements, .. }) => {
            action_terminates_with_fail(statements)
        }
        _ => false,
    }
}

pub(super) fn page_return_kind(statements: &[Statement]) -> Option<HandlerReturnKind> {
    match statements.last()? {
        Statement::ReturnHtml(_) => Some(HandlerReturnKind::Html),
        Statement::ReturnJson(_)
        | Statement::ReturnJsonProjection(_)
        | Statement::ReturnTypedJson { .. } => Some(HandlerReturnKind::Json),
        Statement::Fail(_) => None,
        Statement::Resource { statements, .. } => page_return_kind(statements),
        Statement::Authorize(_)
        | Statement::CanonicalSlug { .. }
        | Statement::Let { .. }
        | Statement::LetValidated { .. }
        | Statement::Set { .. }
        | Statement::While { .. }
        | Statement::If { .. }
        | Statement::Match { .. }
        | Statement::F32ArraySet { .. }
        | Statement::StringDictSet { .. }
        | Statement::PureCall { .. }
        | Statement::LetQuery { .. }
        | Statement::LetOutboundStatus { .. } => None,
    }
}

pub(super) fn action_return_kind(statements: &[ActionStatement]) -> Option<HandlerReturnKind> {
    match statements.last()? {
        ActionStatement::ReturnRedirect(_) => Some(HandlerReturnKind::Redirect),
        ActionStatement::ReturnJson(_)
        | ActionStatement::ReturnJsonProjection(_)
        | ActionStatement::ReturnTypedJson { .. } => Some(HandlerReturnKind::Json),
        ActionStatement::Fail(_) => None,
        ActionStatement::Resource { statements, .. } => action_return_kind(statements),
        ActionStatement::Authorize(_)
        | ActionStatement::Let { .. }
        | ActionStatement::LetValidated { .. }
        | ActionStatement::Set { .. }
        | ActionStatement::While { .. }
        | ActionStatement::If { .. }
        | ActionStatement::Match { .. }
        | ActionStatement::F32ArraySet { .. }
        | ActionStatement::StringDictSet { .. }
        | ActionStatement::LetQuery { .. }
        | ActionStatement::LetOutboundStatus { .. }
        | ActionStatement::Transaction { .. }
        | ActionStatement::Flash(_) => None,
    }
}

pub(super) fn compute_uses_request_state(statements: &[ComputeStatement]) -> Option<&str> {
    for statement in statements {
        let hit = match statement {
            ComputeStatement::Let { expr, .. } | ComputeStatement::Set { expr, .. } => {
                expr_uses_request_state(expr)
            }
            ComputeStatement::F32ArraySet { index, value, .. } => {
                expr_uses_request_state(index).or_else(|| expr_uses_request_state(value))
            }
            ComputeStatement::StringDictSet { key, value, .. } => {
                expr_uses_request_state(key).or_else(|| expr_uses_request_state(value))
            }
            ComputeStatement::PureCall { args, .. } => {
                args.iter().find_map(expr_uses_request_state)
            }
            ComputeStatement::ReturnStruct { fields, .. } => fields
                .iter()
                .find_map(|field| expr_uses_request_state(&field.expr)),
            ComputeStatement::ReturnOption { value } => {
                value.as_ref().and_then(expr_uses_request_state)
            }
            ComputeStatement::ReturnResult { value, .. } => expr_uses_request_state(value),
            ComputeStatement::Return(_) => None,
            ComputeStatement::While {
                condition,
                statements,
            } => expr_uses_request_state(condition)
                .or_else(|| compute_uses_request_state(statements)),
            ComputeStatement::If {
                condition,
                statements,
            } => expr_uses_request_state(condition)
                .or_else(|| compute_uses_request_state(statements)),
        };
        if hit.is_some() {
            return hit;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_source;
    use language_core::PageBody;

    #[test]
    fn pure_if_else_is_lowered_with_single_condition_evaluation() {
        let src = r#"
fn choose(value: i64, enabled: bool) -> i64 {
    let mut out = 0;
    if enabled {
        out = value;
    } else {
        out = value + 1;
    }
    return out;
}

#[page] fn test(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(0));
}
route test GET "/test" public => test;
"#;
        let p = compile_source(src).unwrap();
        let function = p.pure_function("choose").unwrap();
        assert!(function.body.iter().any(|stmt| matches!(
            stmt,
            ComputeStatement::Let { name, .. } if name.starts_with("__velran_if_condition_")
        )));
        assert_eq!(
            function
                .body
                .iter()
                .filter(|stmt| matches!(stmt, ComputeStatement::If { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn pure_else_if_chain_is_supported() {
        let src = r#"
fn classify(value: i64) -> i64 {
    let mut out = 0;
    if value < 0 {
        out = -1;
    } else if value == 0 {
        out = 0;
    } else {
        out = 1;
    }
    return out;
}

#[page] fn test(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(0));
}
route test GET "/test" public => test;
"#;
        let p = compile_source(src).unwrap();
        let function = p.pure_function("classify").unwrap();
        assert!(function.body.iter().any(|stmt| matches!(
            stmt,
            ComputeStatement::Let { name, .. } if name.starts_with("__velran_if_condition_")
        )));
    }

    #[test]
    fn immutable_string_borrows_are_accepted_for_string_builtins_only() {
        let src = r#"
fn pick(text: &str) -> String {
    let tail = substring(&text, 1);
    let ch = charAt(&tail, 0);
    return ch;
}

#[page] fn test(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(0));
}
route test GET "/test" public => test;
"#;
        compile_source(src).unwrap();

        let bad = r#"
fn bad(value: i64) -> bool {
    return regexMatch(&value, "x");
}

#[page] fn test(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(0));
}
route test GET "/test" public => test;
"#;
        let err = compile_source(bad).unwrap_err().to_string();
        assert!(err.contains("line 3:"), "unexpected diagnostic: {err}");
        assert!(
            err.contains("does not accept an immutable borrow"),
            "unexpected diagnostic: {err}"
        );
    }

    #[test]
    fn compiles_budgeted_while_with_scalar_and_array_set() {
        let src = r#"
#[page] fn test(ctx: PageContext) -> Result<Json, PageError> {
    let i = 0;
    let a = vec![0.0f32; 4];
    while i < a.len() {
        a[i] = 1.5f32;
        i = i + 1;
    }
    return Ok(json(i));
}
route test GET "/test" public => test;
"#;
        let p = compile_source(src).unwrap();
        let page = p.page("test").unwrap();
        let PageBody::Statements(statements) = &page.body;
        assert!(
            statements
                .iter()
                .any(|s| matches!(s, Statement::While { .. }))
        );
    }

    #[test]
    fn comparisons_are_typed_bool_and_mixed_numeric_types_are_rejected() {
        let p = Program::default();
        let known = HashMap::new();
        let e = parse_expr("3 < 4", &p).unwrap();
        assert_eq!(infer_expr_type(&e, &known, &p).unwrap(), ValueType::Bool);
        let mixed = parse_expr("3 < 4.0f32", &p).unwrap();
        assert!(infer_expr_type(&mixed, &known, &p).is_err());
    }
}

#[cfg(test)]
mod if_tests {
    use super::*;
    use crate::compile_source;
    use language_core::{BuiltinFunction, PageBody};

    #[test]
    fn compiles_if_with_rust_cast_and_nested_compute() {
        let src = r#"
#[page] fn test(ctx: PageContext) -> Result<Json, PageError> {
    let i = 3;
    let x = i as f32;
    if i < 4 {
        i = i + 1;
    }
    return Ok(json(i));
}
route test GET "/test" public => test;
"#;
        let p = compile_source(src).unwrap();
        let page = p.page("test").unwrap();
        let PageBody::Statements(statements) = &page.body;
        assert!(statements.iter().any(|s| matches!(s, Statement::If { .. })));
        assert!(statements.iter().any(|s| matches!(s, Statement::Let { name, expr: Expr::Builtin { function: BuiltinFunction::ToF32, .. } } if name == "x")));
    }
    #[test]
    fn rejects_legacy_tof32_surface_syntax() {
        let src = r#"
#[page] fn test(ctx: PageContext) -> Result<Json, PageError> {
    let i = 3;
    let x = toF32(i);
    return Ok(json(i));
}
route test GET "/test" public => test;
"#;
        let err = compile_source(src).unwrap_err();
        assert!(
            err.to_string()
                .contains("legacy `toF32(...)` syntax is not supported")
        );
    }
}
