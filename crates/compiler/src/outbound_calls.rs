use crate::diagnostics::CompileError;
use crate::expression::{infer_static_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::StaticType;
use crate::module_namespace::resolve;
use crate::source_syntax::split_top_level;
use language_core::{DataSensitivity, Expr, OutboundCall, OutboundMethod, Program, ValueType};
use std::collections::HashMap;

pub(super) fn parse(
    rhs: &str,
    namespace: &str,
    program: &Program,
    known: &HashMap<String, StaticType>,
    allow_post: bool,
) -> Result<Option<(OutboundCall, StaticType)>, CompileError> {
    let rhs = rhs.trim().strip_suffix('?').unwrap_or(rhs.trim()).trim();
    let Some(open) = rhs.find('(') else {
        return Ok(None);
    };
    if !rhs.ends_with(')') {
        return Ok(None);
    }
    let receiver_and_method = rhs[..open].trim();
    let Some((source_integration, method_name)) = receiver_and_method.rsplit_once('.') else {
        return Ok(None);
    };
    let method = match method_name {
        "get" => OutboundMethod::Get,
        "postJson" => OutboundMethod::PostJson,
        _ => return Ok(None),
    };
    if matches!(method, OutboundMethod::PostJson) && !allow_post {
        return Err(CompileError::security(
            "SEC-SSRF-006",
            "page handlers cannot perform outbound POST effects",
            Some("move the state-changing outbound call to an action handler; page handlers may only use integration GET calls".into()),
        ));
    }
    let integration_name = resolve(namespace, source_integration.trim());
    let integration = match program.integration(&integration_name) {
        Some(value) => value,
        None => return Ok(None),
    };
    let args = split_top_level(&rhs[open + 1..rhs.len() - 1], ',');
    let expected = match method {
        OutboundMethod::Get => 1,
        OutboundMethod::PostJson => 2,
    };
    if args.len() != expected {
        return Err(CompileError::Syntax(format!(
            "integration call `{receiver_and_method}` expects {expected} argument(s)"
        )));
    }
    let path_expr = parse_expr_in_namespace(args[0].trim(), namespace, program)?;
    let Expr::String(path) = path_expr else {
        return Err(CompileError::security(
            "SEC-SSRF-004",
            format!("integration `{source_integration}` request path must be a compiler-known string literal"),
            Some("use a rooted relative literal such as `/v1/items`; dynamic URLs, schemes, hosts, and protocol-relative paths are not accepted".into()),
        ));
    };
    validate_path(source_integration.trim(), &path)?;

    let body = if matches!(method, OutboundMethod::PostJson) {
        let expr = parse_expr_in_namespace(args[1].trim(), namespace, program)?;
        validate_expr(&expr, known, program)?;
        validate_public_json_body(source_integration.trim(), &expr, known, program)?;
        Some(expr)
    } else {
        None
    };
    Ok(Some((
        OutboundCall {
            integration: integration_name,
            egress_target: integration.egress_target.clone(),
            method,
            path,
            body,
        },
        StaticType::trusted_scalar(ValueType::Int),
    )))
}

fn validate_path(integration: &str, path: &str) -> Result<(), CompileError> {
    if path.is_empty()
        || path.len() > 2048
        || !path.starts_with('/')
        || path.starts_with("//")
        || path.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0 | b' '))
        || path.contains("://")
        || has_unsafe_path_segments(path)
    {
        return Err(CompileError::security(
            "SEC-SSRF-005",
            format!("integration `{integration}` has unsafe outbound path literal `{path}`"),
            Some("use a rooted, canonical, header-safe relative path without dot segments or encoded separators; the trusted egress policy owns scheme, host, port, DNS, and TLS authority".into()),
        ));
    }
    Ok(())
}

fn has_unsafe_path_segments(path_and_query: &str) -> bool {
    let path = path_and_query
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(path_and_query);
    path.split('/').any(|segment| {
        if segment.is_empty() {
            return false;
        }
        let lower = segment.to_ascii_lowercase();
        lower == "."
            || lower == ".."
            || lower.starts_with('.')
            || lower.contains("%2e")
            || lower.contains("%2f")
            || lower.contains("%5c")
    })
}

fn validate_public_json_body(
    integration: &str,
    expr: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    let ty = infer_static_expr_type(expr, known, program)?;
    let Some(scalar) = ty.scalar() else {
        return Err(CompileError::security(
            "SEC-DATA-010",
            format!("integration `{integration}` outbound JSON body must be an explicit public scalar/collection value"),
            Some("do not send complete models or implicit projections to external systems; construct an explicit public value first".into()),
        ));
    };
    if scalar.sensitivity >= DataSensitivity::Sensitive
        || matches!(
            scalar.value_type,
            ValueType::Credential(_) | ValueType::Upload
        )
    {
        return Err(CompileError::security(
            "SEC-DATA-011",
            format!("integration `{integration}` outbound JSON body cannot contain unredacted Sensitive<T>, Secret<T>, credential-purpose, or Upload data"),
            Some("send an explicit public projection/value; secret-bearing integrations require a separate purpose-specific trusted capability".into()),
        ));
    }
    Ok(())
}
