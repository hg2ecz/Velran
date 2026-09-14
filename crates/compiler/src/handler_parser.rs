use crate::diagnostics::CompileError;
use crate::module_namespace::qualify;
use crate::source_syntax::{function_bounds, line_number};
use crate::{action_statements, control_flow, declarations, page_statements};
use language_core::{ActionBody, ActionFunction, PageBody, PageFunction, Program};

pub(super) fn parse_pages(
    source: &str,
    source_name: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0;
    while let Some(rel) = source[off..].find("page fn ") {
        let keyword = off + rel;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            off = keyword + 8;
            continue;
        }
        let start = keyword + 8;
        let (name, sig_open, sig_close, body_open, body_close) =
            function_bounds(source, start, "page")?;
        let symbol_name = qualify(namespace, &name);
        if p.page(&symbol_name).is_some()
            || p.action(&symbol_name).is_some()
            || p.component(&symbol_name).is_some()
            || p.layout(&symbol_name).is_some()
        {
            return Err(CompileError::DuplicateHandler(name));
        }
        let (declared_return, security, declared_effects, response_schema) =
            crate::handler_signature::parse_handler_contract(
                "page",
                &name,
                &source[sig_close + 1..body_open],
                namespace,
                p,
            )?;
        let (params, needs_db, request_aliases) = crate::handler_signature::parse_handler_params(
            &name,
            "PageContext",
            &source[sig_open + 1..sig_close],
            namespace,
            p,
        )?;
        let base_line = line_number(source, body_open + 1);
        let body = rewrite_request_aliases(&source[body_open + 1..body_close], &request_aliases);
        let statements = page_statements::parse_page_statements(
            &symbol_name,
            namespace,
            &body,
            &params,
            p,
            source_name,
            base_line,
            true,
        )?;
        if !control_flow::page_return_matches(&statements, declared_return) {
            return Err(CompileError::Syntax(format!(
                "page `{name}` return statement does not match declared return type"
            )));
        }
        crate::response_contract::validate_page_response_schema(
            &statements,
            response_schema.as_deref(),
            &name,
        )?;
        let inferred_effects = crate::effect_security::collect_page(&statements);
        crate::effect_security::validate_network_capabilities(
            "page",
            &name,
            &declared_effects,
            &inferred_effects,
        )?;
        let effects = crate::effect_security::merge(declared_effects, inferred_effects);
        crate::effect_security::validate_db_capability("page", &name, needs_db, &effects)?;
        p.pages.push(PageFunction {
            name: symbol_name,
            params,
            needs_db,
            effects,
            security,
            body: PageBody::Statements(statements),
        });
        off = body_close + 1;
    }
    Ok(())
}

pub(super) fn parse_actions(
    source: &str,
    source_name: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0;
    while let Some(rel) = source[off..].find("action fn ") {
        let keyword = off + rel;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            off = keyword + 10;
            continue;
        }
        let start = keyword + 10;
        let (name, sig_open, sig_close, body_open, body_close) =
            function_bounds(source, start, "action")?;
        let symbol_name = qualify(namespace, &name);
        if p.page(&symbol_name).is_some()
            || p.action(&symbol_name).is_some()
            || p.component(&symbol_name).is_some()
            || p.layout(&symbol_name).is_some()
        {
            return Err(CompileError::DuplicateHandler(name));
        }
        let (declared_return, security, declared_effects, response_schema) =
            crate::handler_signature::parse_handler_contract(
                "action",
                &name,
                &source[sig_close + 1..body_open],
                namespace,
                p,
            )?;
        let (params, needs_db, request_aliases) = crate::handler_signature::parse_handler_params(
            &name,
            "ActionContext",
            &source[sig_open + 1..sig_close],
            namespace,
            p,
        )?;
        let base_line = line_number(source, body_open + 1);
        let body = rewrite_request_aliases(&source[body_open + 1..body_close], &request_aliases);
        let statements = action_statements::parse_action_statements(
            &symbol_name,
            namespace,
            &body,
            &params,
            p,
            source_name,
            base_line,
            true,
        )?;
        if !control_flow::action_return_matches(&statements, declared_return) {
            return Err(CompileError::Syntax(format!(
                "action `{name}` return statement does not match declared return type"
            )));
        }
        crate::response_contract::validate_action_response_schema(
            &statements,
            response_schema.as_deref(),
            &name,
        )?;
        let inferred_effects = crate::effect_security::collect_action(&statements);
        crate::effect_security::validate_network_capabilities(
            "action",
            &name,
            &declared_effects,
            &inferred_effects,
        )?;
        let effects = crate::effect_security::merge(declared_effects, inferred_effects);
        crate::effect_security::validate_db_capability("action", &name, needs_db, &effects)?;
        p.actions.push(ActionFunction {
            name: symbol_name,
            params,
            needs_db,
            effects,
            security,
            body: ActionBody::Statements(statements),
        });
        off = body_close + 1;
    }
    Ok(())
}

fn rewrite_request_aliases(body: &str, aliases: &[(String, String)]) -> String {
    if aliases.is_empty() {
        return body.to_string();
    }
    let mut out = body.to_string();
    for (from, to) in aliases {
        out = replace_code_token(&out, from, to);
    }
    out
}

fn replace_code_token(input: &str, from: &str, to: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    let mut quote: Option<u8> = None;
    while i < bytes.len() {
        if let Some(q) = quote {
            out.push(bytes[i] as char);
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                i += 1;
                out.push(bytes[i] as char);
            } else if bytes[i] == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if matches!(bytes[i], b'\"' | b'\'') {
            quote = Some(bytes[i]);
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        if input[i..].starts_with(from) {
            let before_ok =
                i == 0 || !matches!(bytes[i-1], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_');
            let end = i + from.len();
            let after_ok = end == bytes.len()
                || !matches!(bytes[end], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_');
            if before_ok && after_ok {
                out.push_str(to);
                i = end;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}
