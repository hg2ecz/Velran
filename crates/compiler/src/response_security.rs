use crate::diagnostics::CompileError;
use crate::expression_security::infer_static_expr_type;
use crate::handler_types::StaticType;
use crate::scalar_security::DisclosureEvidence;
use language_core::{Expr, Program, ValueType};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(super) enum ResponseBoundary {
    Json,
    Html,
}

pub(super) fn validate_response_expression(
    expression: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
    boundary: ResponseBoundary,
) -> Result<(), CompileError> {
    let static_type = infer_static_expr_type(expression, known, program)?;
    if matches!(
        static_type,
        StaticType::Model(_) | StaticType::OptionalModel(_) | StaticType::ListModel(_)
    ) {
        return Err(model_exposure_error(boundary));
    }
    if let Some(scalar) = static_type.scalar() {
        if matches!(scalar.value_type, ValueType::Credential(_)) {
            return Err(credential_error(boundary));
        }
    }
    if let Some(scalar) = static_type.scalar() {
        if scalar.sensitivity == language_core::DataSensitivity::Secret {
            return Err(secret_error(boundary));
        }
        if scalar.sensitivity >= language_core::DataSensitivity::Sensitive
            && scalar.disclosure != DisclosureEvidence::Authorized
        {
            return Err(sensitive_without_authorization_error(boundary));
        }
    }
    Ok(())
}

fn secret_error(boundary: ResponseBoundary) -> CompileError {
    match boundary {
        ResponseBoundary::Json => CompileError::security(
            "SEC-DATA-001",
            "Secret<T> data cannot be serialized into a JSON response",
            Some("return a public DTO or a redacted/non-secret value instead".into()),
        ),
        ResponseBoundary::Html => CompileError::security(
            "SEC-DATA-002",
            "Secret<T> data cannot be rendered into HTML",
            Some("render a deliberately derived public value instead".into()),
        ),
    }
}

fn sensitive_without_authorization_error(boundary: ResponseBoundary) -> CompileError {
    let target = match boundary {
        ResponseBoundary::Json => "JSON response",
        ResponseBoundary::Html => "HTML response",
    };
    CompileError::security(
        "SEC-A01-004",
        format!("Sensitive<T> data cannot enter a {target} without object authorization proof"),
        Some("authorize the loaded object before deriving or returning the sensitive value".into()),
    )
}

fn model_exposure_error(boundary: ResponseBoundary) -> CompileError {
    let target = match boundary {
        ResponseBoundary::Json => "JSON response",
        ResponseBoundary::Html => "HTML response",
    };
    CompileError::security(
        "SEC-DATA-004",
        format!("model values cannot enter a {target} without an explicit public projection"),
        Some("use expose(model, field, ...) and list only fields intended for the response".into()),
    )
}

fn credential_error(boundary: ResponseBoundary) -> CompileError {
    let target = match boundary {
        ResponseBoundary::Json => "JSON response",
        ResponseBoundary::Html => "HTML response",
    };
    CompileError::security(
        "SEC-A04-004",
        format!("credential-purpose values cannot enter a {target} through a generic response expression"),
        Some("use a dedicated credential/session/CSRF protocol primitive; generic rendering and serialization deliberately reject credentials".into()),
    )
}
