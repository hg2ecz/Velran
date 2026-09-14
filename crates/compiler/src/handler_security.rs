use crate::diagnostics::CompileError;
use crate::module_namespace::resolve;
use language_core::{HandlerSecurityContract, Program};

pub(super) fn parse_requirements(
    handler: &str,
    raw: &str,
    namespace: &str,
    program: &Program,
) -> Result<HandlerSecurityContract, CompileError> {
    if raw.is_empty() {
        return Err(requirements_syntax(handler));
    }
    let parts: Vec<&str> = raw.split('+').map(str::trim).collect();
    if parts.iter().any(|part| part.is_empty()) || parts.len() > 2 {
        return Err(requirements_syntax(handler));
    }
    let mut contract = HandlerSecurityContract::default();
    for part in parts {
        apply_requirement(handler, part, namespace, program, &mut contract)?;
    }
    Ok(contract)
}

pub(super) fn parse_critical(
    handler: &str,
    source_name: &str,
    namespace: &str,
    program: &Program,
) -> Result<HandlerSecurityContract, CompileError> {
    if source_name.split_whitespace().count() != 1 {
        return Err(CompileError::Syntax(format!(
            "handler `{handler}` critical contract is `critical <Operation>`"
        )));
    }
    let name = resolve(namespace, source_name);
    let operation = program.critical_operation(&name).ok_or_else(|| {
        CompileError::security(
            "SEC-A06-002",
            format!("handler `{handler}` references unknown critical operation `{source_name}`"),
            Some("declare it once with `critical Name { ... }`".into()),
        )
    })?;
    Ok(HandlerSecurityContract {
        required_permission: operation.required_permission.clone(),
        mfa_required: operation.mfa_required,
        critical_operation: Some(operation.name.clone()),
    })
}

fn apply_requirement(
    handler: &str,
    part: &str,
    namespace: &str,
    program: &Program,
    contract: &mut HandlerSecurityContract,
) -> Result<(), CompileError> {
    if part == "mfa" {
        if contract.mfa_required {
            return Err(requirements_syntax(handler));
        }
        contract.mfa_required = true;
        return Ok(());
    }
    if contract.required_permission.is_some() || part.split_whitespace().count() != 1 {
        return Err(requirements_syntax(handler));
    }
    let permission = resolve(namespace, part);
    if program.permission(&permission).is_none() {
        return Err(CompileError::security(
            "SEC-A01-020",
            format!("handler `{handler}` requires unknown permission `{part}`"),
            Some("declare the permission once with `permission Name { role RoleName }`, then reference that permission from the handler".into()),
        ));
    }
    contract.required_permission = Some(permission);
    Ok(())
}

fn requirements_syntax(handler: &str) -> CompileError {
    CompileError::Syntax(format!(
        "handler `{handler}` security contract is `requires <Permission>`, `requires mfa`, or `requires <Permission> + mfa`"
    ))
}
