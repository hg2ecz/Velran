use crate::declarations;
use crate::diagnostics::CompileError;
use crate::module_namespace::{qualify, resolve};
use crate::source_syntax::{is_identifier, matching_brace, read_ident};
use language_core::{CriticalOperation, Program};

pub(super) fn parse_critical_operations(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    let mut offset = 0usize;
    while let Some(relative) = source[offset..].find("critical ") {
        let keyword = offset + relative;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            offset = keyword + "critical ".len();
            continue;
        }
        let start = keyword + "critical ".len();
        let name = read_ident(source, start)
            .ok_or_else(|| CompileError::Syntax("critical operation name expected".into()))?;
        validate_name(&name)?;
        let symbol = qualify(namespace, &name);
        if program.critical_operation(&symbol).is_some() {
            return Err(CompileError::Syntax(format!(
                "duplicate critical operation `{name}`"
            )));
        }
        let open = skip_to_brace(source, start + name.len(), &name)?;
        let close = matching_brace(source, open).ok_or_else(|| {
            CompileError::Syntax(format!("critical operation `{name}` body is unclosed"))
        })?;
        let mut operation = parse_body(&name, &source[open + 1..close], namespace, program)?;
        operation.name = symbol;
        program.critical_operations.push(operation);
        offset = close + 1;
    }
    Ok(())
}

fn parse_body(
    name: &str,
    body: &str,
    namespace: &str,
    program: &Program,
) -> Result<CriticalOperation, CompileError> {
    let mut operation = CriticalOperation {
        name: String::new(),
        required_permission: None,
        mfa_required: false,
        transaction_required: false,
        audit_required: false,
        idempotency_required: false,
        required_security_event: None,
    };
    let mut saw_requirement = false;
    for line in body.lines() {
        let clean = line
            .split_once("//")
            .map_or(line, |(before, _)| before)
            .trim();
        let clean = clean.trim_end_matches(';').trim();
        if clean.is_empty() {
            continue;
        }
        saw_requirement = true;
        match clean {
            "mfa" => set_once(&mut operation.mfa_required, name, "mfa")?,
            "transaction" => set_once(&mut operation.transaction_required, name, "transaction")?,
            "audit" => set_once(&mut operation.audit_required, name, "audit")?,
            "idempotency" => set_once(&mut operation.idempotency_required, name, "idempotency")?,
            _ if clean.starts_with("audit ") => {
                if operation.audit_required {
                    return Err(duplicate_requirement(name, "audit"));
                }
                let source_name = clean["audit ".len()..].trim();
                if !is_identifier(source_name) {
                    return Err(CompileError::Syntax(format!(
                        "critical operation `{name}` has invalid security event `{source_name}`"
                    )));
                }
                let event_name = resolve(namespace, source_name);
                if program.security_event(&event_name).is_none() {
                    return Err(CompileError::security(
                        "SEC-A09-005",
                        format!("critical operation `{name}` references unknown security event `{source_name}`"),
                        Some("declare it with `security event Name for Model;` before the critical operation".into()),
                    ));
                }
                operation.audit_required = true;
                operation.required_security_event = Some(event_name);
            }
            _ if clean.starts_with("permission ") => {
                if operation.required_permission.is_some() {
                    return Err(duplicate_requirement(name, "permission"));
                }
                let source_name = clean["permission ".len()..].trim();
                if !is_identifier(source_name) {
                    return Err(CompileError::Syntax(format!(
                        "critical operation `{name}` has invalid permission `{source_name}`"
                    )));
                }
                let permission = resolve(namespace, source_name);
                if program.permission(&permission).is_none() {
                    return Err(CompileError::security(
                        "SEC-A06-001",
                        format!("critical operation `{name}` references unknown permission `{source_name}`"),
                        Some("declare the permission before using it in a critical operation contract".into()),
                    ));
                }
                operation.required_permission = Some(permission);
            }
            _ => {
                return Err(CompileError::Syntax(format!(
                    "critical operation `{name}` entries are `permission <Name>`, `mfa`, `transaction`, `idempotency`, `audit`, or `audit <SecurityEvent>`"
                )));
            }
        }
    }
    if !saw_requirement {
        return Err(CompileError::Syntax(format!(
            "critical operation `{name}` must declare at least one security requirement"
        )));
    }
    Ok(operation)
}

fn set_once(value: &mut bool, name: &str, requirement: &str) -> Result<(), CompileError> {
    if *value {
        return Err(duplicate_requirement(name, requirement));
    }
    *value = true;
    Ok(())
}

fn duplicate_requirement(name: &str, requirement: &str) -> CompileError {
    CompileError::Syntax(format!(
        "critical operation `{name}` contains duplicate `{requirement}` requirement"
    ))
}

fn validate_name(name: &str) -> Result<(), CompileError> {
    if !is_identifier(name)
        || !name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return Err(CompileError::Syntax(format!(
            "critical operation `{name}` must start with an uppercase ASCII letter"
        )));
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
            "critical operation `{name}` must use `critical Name {{ ... }}`"
        )));
    }
    Ok(cursor)
}
