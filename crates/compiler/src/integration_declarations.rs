use crate::declarations;
use crate::diagnostics::CompileError;
use crate::module_namespace::qualify;
use crate::source_syntax::{is_identifier, matching_brace, read_ident};
use language_core::{Integration, Program};

pub(super) fn parse_integrations(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    let mut offset = 0usize;
    while let Some(relative) = source[offset..].find("integration ") {
        let keyword = offset + relative;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            offset = keyword + "integration ".len();
            continue;
        }
        let start = keyword + "integration ".len();
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("integration name expected".into()))?;
        validate_symbol_name(&name)?;
        let symbol = qualify(namespace, &name);
        if program.integration(&symbol).is_some() {
            return Err(CompileError::Syntax(format!(
                "duplicate integration `{name}`"
            )));
        }
        let open = skip_to_brace(source, start + name.len(), &name)?;
        let close = matching_brace(source, open).ok_or_else(|| {
            CompileError::Syntax(format!("integration `{name}` body is unclosed"))
        })?;
        let egress_target = parse_body(&name, &source[open + 1..close])?;
        program.integrations.push(Integration {
            name: symbol,
            egress_target,
        });
        offset = close + 1;
    }
    Ok(())
}

fn parse_body(name: &str, body: &str) -> Result<String, CompileError> {
    let mut egress_target = None;
    for line in body.lines() {
        let clean = line
            .split_once("//")
            .map_or(line, |(before, _)| before)
            .trim();
        let clean = clean.trim_end_matches(';').trim();
        if clean.is_empty() {
            continue;
        }
        if let Some(value) = clean.strip_prefix("egress ") {
            if egress_target.is_some() {
                return Err(CompileError::Syntax(format!(
                    "integration `{name}` contains duplicate `egress` entry"
                )));
            }
            validate_egress_target(name, value)?;
            egress_target = Some(value.to_string());
        } else {
            return Err(CompileError::Syntax(format!(
                "integration `{name}` entries must use `egress <target>`"
            )));
        }
    }
    egress_target.ok_or_else(|| {
        CompileError::security(
            "SEC-SSRF-001",
            format!("integration `{name}` has no named egress target"),
            Some("add `egress <target>`; outbound hosts and CIDRs are controlled by the trusted server egress policy".into()),
        )
    })
}

fn validate_symbol_name(name: &str) -> Result<(), CompileError> {
    if !is_identifier(name)
        || !name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return Err(CompileError::Syntax(format!(
            "integration `{name}` must start with an uppercase ASCII letter"
        )));
    }
    Ok(())
}

fn validate_egress_target(name: &str, value: &str) -> Result<(), CompileError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(CompileError::security(
            "SEC-SSRF-002",
            format!("integration `{name}` has invalid egress target `{value}`"),
            Some("use a short named target such as `payments`; hostnames and URLs do not belong in Velran source".into()),
        ));
    }
    Ok(())
}

fn skip_to_brace(source: &str, mut cursor: usize, name: &str) -> Result<usize, CompileError> {
    while source
        .as_bytes()
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += 1;
    }
    if source.as_bytes().get(cursor) != Some(&b'{') {
        return Err(CompileError::Syntax(format!(
            "integration `{name}` must use `integration Name {{ ... }}`"
        )));
    }
    Ok(cursor)
}
