use crate::declarations;
use crate::diagnostics::CompileError;
use crate::module_namespace::qualify;
use crate::source_syntax::{is_identifier, matching_brace, read_ident};
use language_core::{Permission, Program};

pub(super) fn parse_permissions(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    let mut offset = 0usize;
    while let Some(relative) = source[offset..].find("permission ") {
        let keyword = offset + relative;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            offset = keyword + "permission ".len();
            continue;
        }
        let start = keyword + "permission ".len();
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("permission name expected".into()))?;
        validate_permission_name(&name)?;
        let symbol_name = qualify(namespace, &name);
        if program.permission(&symbol_name).is_some() {
            return Err(CompileError::Syntax(format!(
                "duplicate permission `{name}`"
            )));
        }
        let after_name = start + name.len();
        let open = skip_whitespace_to_brace(source, after_name, &name)?;
        let close = matching_brace(source, open)
            .ok_or_else(|| CompileError::Syntax(format!("permission `{name}` body is unclosed")))?;
        let roles = parse_roles(&name, &source[open + 1..close])?;
        program.permissions.push(Permission {
            name: symbol_name,
            roles,
        });
        offset = close + 1;
    }
    Ok(())
}

fn validate_permission_name(name: &str) -> Result<(), CompileError> {
    if !is_identifier(name)
        || !name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return Err(CompileError::Syntax(format!(
            "permission `{name}` must start with an uppercase ASCII letter"
        )));
    }
    Ok(())
}

fn skip_whitespace_to_brace(
    source: &str,
    mut cursor: usize,
    permission: &str,
) -> Result<usize, CompileError> {
    while source
        .as_bytes()
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += 1;
    }
    if source.as_bytes().get(cursor) != Some(&b'{') {
        return Err(CompileError::Syntax(format!(
            "permission `{permission}` must use `permission Name {{ role RoleName ... }}`"
        )));
    }
    Ok(cursor)
}

fn parse_roles(permission: &str, body: &str) -> Result<Vec<String>, CompileError> {
    let mut roles = Vec::new();
    for line in body.lines() {
        let clean = line
            .split_once("//")
            .map_or(line, |(before, _)| before)
            .trim();
        if clean.is_empty() {
            continue;
        }
        let role = clean.strip_prefix("role ").map(str::trim).ok_or_else(|| {
            CompileError::Syntax(format!(
                "permission `{permission}` entries must use `role <RoleName>`"
            ))
        })?;
        let role = role.trim_end_matches(';').trim();
        if !is_identifier(role) {
            return Err(CompileError::Syntax(format!(
                "permission `{permission}` has invalid role `{role}`"
            )));
        }
        if roles.iter().any(|existing| existing == role) {
            return Err(CompileError::Syntax(format!(
                "permission `{permission}` contains duplicate role `{role}`"
            )));
        }
        roles.push(role.to_string());
    }
    if roles.is_empty() {
        return Err(CompileError::Syntax(format!(
            "permission `{permission}` requires at least one role"
        )));
    }
    Ok(roles)
}
