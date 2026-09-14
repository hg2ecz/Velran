use crate::diagnostics::CompileError;
use crate::expression_security::infer_static_expr_type;
use crate::handler_types::StaticType;
use language_core::{DataSensitivity, Expr, FunctionParam, Program, ValueType};
use std::collections::HashMap;

pub(super) fn validate_query_argument(
    query: &str,
    param: &FunctionParam,
    argument: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    let actual = infer_static_expr_type(argument, known, program)?
        .scalar()
        .map(|scalar| scalar.sensitivity)
        .unwrap_or(DataSensitivity::Public);

    match (param.sensitivity, actual) {
        (DataSensitivity::Secret, DataSensitivity::Secret) => Ok(()),
        (DataSensitivity::Secret, _) => Err(CompileError::security(
            "SEC-DATA-007",
            format!(
                "query `{query}` parameter `{}` requires Secret<T> data",
                param.name
            ),
            Some("pass a value that is statically classified Secret<T>; do not manufacture secrets with casts or string conversion".into()),
        )),
        (_, DataSensitivity::Secret) => Err(CompileError::security(
            "SEC-DATA-006",
            format!(
                "Secret<T> data cannot be passed to non-secret query parameter `{}.{}`",
                query, param.name
            ),
            Some(format!(
                "declare the sink explicitly as `{}: Secret<...>` if this query is intentionally allowed to consume the secret",
                param.name
            )),
        )),
        _ => Ok(()),
    }
}

pub(super) fn reject_audit_secret(
    expression: &Expr,
    label: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    let scalar = infer_static_expr_type(expression, known, program)?.scalar();
    if scalar
        .as_ref()
        .is_some_and(|value| matches!(value.value_type, ValueType::Credential(_)))
    {
        return Err(CompileError::security(
            "SEC-A04-005",
            format!("{label} cannot contain credential-purpose data"),
            Some("audit stable object identifiers and security event metadata, never passwords, hashes, tokens, or cryptographic keys".into()),
        ));
    }
    let sensitivity = scalar
        .map(|scalar| scalar.sensitivity)
        .unwrap_or(DataSensitivity::Public);
    if sensitivity >= DataSensitivity::Sensitive {
        return Err(CompileError::security(
            "SEC-DATA-008",
            format!("{label} cannot contain Sensitive<T> or Secret<T> data"),
            Some("audit stable identifiers, enum/status values, or deliberately redacted public data instead".into()),
        ));
    }
    Ok(())
}
