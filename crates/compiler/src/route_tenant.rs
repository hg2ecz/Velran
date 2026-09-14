use crate::diagnostics::CompileError;
use crate::source_syntax::is_identifier;
use language_core::{Program, RouteAuth, ValueType};
use std::collections::HashMap;

pub(super) fn parse_tenant_field(
    tokens: &[String],
    cursor: &mut usize,
    route: &str,
) -> Result<Option<String>, CompileError> {
    if tokens.get(*cursor).map(String::as_str) != Some("tenant") {
        return Ok(None);
    }
    let field = tokens.get(*cursor + 1).ok_or_else(|| {
        CompileError::Syntax(format!(
            "route `{route}` tenant binding requires an input field name"
        ))
    })?;
    if !is_identifier(field) {
        return Err(CompileError::Syntax(format!(
            "route `{route}` tenant binding `{field}` is not a valid identifier"
        )));
    }
    *cursor += 2;
    Ok(Some(field.clone()))
}

pub(super) fn validate_tenant_field(
    route: &str,
    tenant_field: Option<&str>,
    field_types: &HashMap<String, ValueType>,
    auth: &RouteAuth,
    program: &Program,
) -> Result<(), CompileError> {
    let Some(field) = tenant_field else {
        return Ok(());
    };
    if matches!(auth, RouteAuth::Public | RouteAuth::Webhook(_)) {
        return Err(CompileError::security(
            "SEC-A01-032",
            format!("tenant-bound route `{route}` cannot be public"),
            Some("require authenticated access so the runtime can verify the active tenant against trusted membership claims".into()),
        ));
    }
    let ty = field_types.get(field).copied().ok_or_else(|| CompileError::security(
        "SEC-A01-033",
        format!("route `{route}` tenant binding must reference a path/query input `{field}`"),
        Some("bind the active tenant from a typed path (or GET query) field so membership is verified before request-body processing".into()),
    ))?;
    if program.representation_type(ty) != Some(ValueType::String) {
        return Err(CompileError::security(
            "SEC-A01-034",
            format!("route `{route}` tenant input `{field}` must have String representation"),
            Some(
                "use a nominal String domain type such as OrganizationId for tenant identifiers"
                    .into(),
            ),
        ));
    }
    Ok(())
}
