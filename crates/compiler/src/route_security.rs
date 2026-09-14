use crate::diagnostics::CompileError;
use crate::module_namespace::resolve;
use crate::source_syntax::is_identifier;
use language_core::{FormField, Program, RouteAuth, ValidationKind, ValidationRule, ValueType};

pub(super) const DEFAULT_EXTERNAL_STRING_MAX: usize = 4096;
pub(super) const DEFAULT_EXTERNAL_LIST_MAX: usize = 64;

pub(super) fn parse_explicit_access(
    route_name: &str,
    tokens: &[String],
    cursor: &mut usize,
    namespace: &str,
    program: &Program,
) -> Result<RouteAuth, CompileError> {
    match tokens.get(*cursor).map(String::as_str) {
        Some("public") => {
            *cursor += 1;
            Ok(RouteAuth::Public)
        }
        Some("webhook") => {
            let source_name = tokens
                .get(*cursor + 1)
                .ok_or_else(|| CompileError::Syntax("route webhook name expected".into()))?;
            if !is_identifier(source_name) {
                return Err(CompileError::Syntax("invalid route webhook name".into()));
            }
            let name = resolve(namespace, source_name);
            if program.webhook(&name).is_none() {
                return Err(CompileError::security(
                    "SEC-A08-022",
                    format!("route `{route_name}` references unknown webhook `{source_name}`"),
                    Some("declare it once with `webhook Name { verified by SecretName }`".into()),
                ));
            }
            *cursor += 2;
            Ok(RouteAuth::Webhook(name))
        }
        Some("auth") => {
            *cursor += 1;
            let mode = tokens
                .get(*cursor)
                .ok_or_else(|| CompileError::Syntax("route auth mode expected".into()))?;
            match mode.as_str() {
                "user" => {
                    *cursor += 1;
                    Ok(RouteAuth::User)
                }
                "mfa" => {
                    *cursor += 1;
                    Ok(RouteAuth::Mfa)
                }
                "role" => {
                    let role = tokens
                        .get(*cursor + 1)
                        .ok_or_else(|| CompileError::Syntax("route auth role name expected".into()))?
                        .clone();
                    if !is_identifier(&role) {
                        return Err(CompileError::Syntax("invalid route role name".into()));
                    }
                    *cursor += 2;
                    Ok(RouteAuth::Role(role))
                }
                "permission" => {
                    let source_name = tokens
                        .get(*cursor + 1)
                        .ok_or_else(|| CompileError::Syntax("route auth permission name expected".into()))?;
                    if !is_identifier(source_name) {
                        return Err(CompileError::Syntax("invalid route permission name".into()));
                    }
                    let name = resolve(namespace, source_name);
                    let permission = program.permission(&name).ok_or_else(|| {
                        CompileError::security(
                            "SEC-A01-021",
                            format!("route `{route_name}` references unknown permission `{source_name}`"),
                            Some("declare the permission once with `permission Name { role RoleName }`".into()),
                        )
                    })?;
                    *cursor += 2;
                    let mfa = tokens.get(*cursor).map(String::as_str) == Some("mfa");
                    if mfa {
                        *cursor += 1;
                        Ok(RouteAuth::PermissionMfa {
                            name: permission.name.clone(),
                            roles: permission.roles.clone(),
                        })
                    } else {
                        Ok(RouteAuth::Permission {
                            name: permission.name.clone(),
                            roles: permission.roles.clone(),
                        })
                    }
                }
                "critical" => {
                    let source_name = tokens
                        .get(*cursor + 1)
                        .ok_or_else(|| CompileError::Syntax("route auth critical operation name expected".into()))?;
                    if !is_identifier(source_name) {
                        return Err(CompileError::Syntax("invalid critical operation name".into()));
                    }
                    let name = resolve(namespace, source_name);
                    let operation = program.critical_operation(&name).ok_or_else(|| {
                        CompileError::security(
                            "SEC-A06-005",
                            format!("route `{route_name}` references unknown critical operation `{source_name}`"),
                            Some("declare it once with `critical Name { ... }`".into()),
                        )
                    })?;
                    *cursor += 2;
                    match (&operation.required_permission, operation.mfa_required) {
                        (Some(permission_name), true) => {
                            let permission = program.permission(permission_name).expect("validated critical permission");
                            Ok(RouteAuth::PermissionMfa {
                                name: permission.name.clone(),
                                roles: permission.roles.clone(),
                            })
                        }
                        (Some(permission_name), false) => {
                            let permission = program.permission(permission_name).expect("validated critical permission");
                            Ok(RouteAuth::Permission {
                                name: permission.name.clone(),
                                roles: permission.roles.clone(),
                            })
                        }
                        (None, true) => Ok(RouteAuth::Mfa),
                        (None, false) => Ok(RouteAuth::User),
                    }
                }
                _ => Err(CompileError::Syntax(format!(
                    "unknown route auth mode `{mode}`"
                ))),
            }
        }
        _ => Err(CompileError::security(
            "SEC-A01-001",
            format!("route `{route_name}` has no explicit access policy"),
            Some(
                "add `public` for intentionally public endpoints, `webhook <Name>` for verified machine callbacks, or `auth user`, `auth mfa`, `auth role <Role>`, `auth permission <Permission>`, `auth permission <Permission> mfa`, or `auth critical <Operation>`"
                    .into(),
            ),
        )),
    }
}

pub(super) fn apply_default_external_input_bounds(
    fields: impl Iterator<Item = FormField>,
    validations: &mut Vec<ValidationRule>,
) {
    for field in fields {
        let has_bound = |kind: &ValidationKind| match (field.ty, kind) {
            (ValueType::String, ValidationKind::Length { .. }) => true,
            (ValueType::StringList, ValidationKind::Items { .. }) => true,
            _ => false,
        };
        if validations
            .iter()
            .any(|rule| rule.field == field.name && has_bound(&rule.kind))
        {
            continue;
        }
        let kind = match field.ty {
            ValueType::String => ValidationKind::Length {
                min: 0,
                max: DEFAULT_EXTERNAL_STRING_MAX,
            },
            ValueType::StringList => ValidationKind::Items {
                min: 1,
                max: DEFAULT_EXTERNAL_LIST_MAX,
            },
            _ => continue,
        };
        validations.push(ValidationRule {
            field: field.name,
            kind,
        });
    }
}
