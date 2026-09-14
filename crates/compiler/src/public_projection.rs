use crate::diagnostics::CompileError;
use crate::handler_types::StaticType;
use crate::model_security::ModelAccess;
use language_core::{DataSensitivity, Program, ProjectionSourceKind, PublicProjection, ValueType};
use std::collections::{HashMap, HashSet};

pub(super) fn parse_public_projection(
    raw: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<Option<PublicProjection>, CompileError> {
    let Some(inner) = raw
        .strip_prefix("expose(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Ok(None);
    };
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err(CompileError::Syntax(
            "expose(value, field, ...) requires a model value and at least one field".into(),
        ));
    }
    let source = parts[0];
    let (model_name, source_kind, access) = projection_source(source, known)?;
    let model = program
        .model(model_name)
        .ok_or_else(|| CompileError::UnknownModel(model_name.to_string()))?;

    let mut fields = Vec::with_capacity(parts.len() - 1);
    let mut seen = HashSet::new();
    for field_name in &parts[1..] {
        validate_identifier(field_name)?;
        if !seen.insert(*field_name) {
            return Err(CompileError::Syntax(format!(
                "`expose` lists field `{field_name}` more than once"
            )));
        }
        let field = model
            .fields
            .iter()
            .find(|candidate| candidate.name == *field_name)
            .ok_or_else(|| {
                CompileError::Syntax(format!("model `{model_name}` has no field `{field_name}`"))
            })?;
        validate_disclosure(access, model_name, field_name, field.ty, field.sensitivity)?;
        fields.push((*field_name).to_string());
    }

    Ok(Some(PublicProjection {
        source: source.to_string(),
        source_kind,
        model: model_name.to_string(),
        fields,
    }))
}

fn projection_source<'a>(
    source: &str,
    known: &'a HashMap<String, StaticType>,
) -> Result<(&'a str, ProjectionSourceKind, ModelAccess), CompileError> {
    match known.get(source) {
        Some(StaticType::Model(model)) => {
            Ok((&model.name, ProjectionSourceKind::Model, model.access))
        }
        Some(StaticType::OptionalModel(model)) => Ok((
            model,
            ProjectionSourceKind::OptionalModel,
            ModelAccess::Unverified,
        )),
        Some(StaticType::ListModel(model)) => Ok((
            model,
            ProjectionSourceKind::ModelList,
            ModelAccess::Unverified,
        )),
        _ => Err(CompileError::security(
            "SEC-DATA-004",
            format!(
                "`expose` requires a model, optional model, or model list; `{source}` is not one"
            ),
            Some("load a model value first, then expose an explicit field list".into()),
        )),
    }
}

fn validate_identifier(field: &str) -> Result<(), CompileError> {
    if !field.is_empty()
        && field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && field
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    {
        return Ok(());
    }
    Err(CompileError::Syntax(format!(
        "`expose` field `{field}` must be an identifier"
    )))
}

fn validate_disclosure(
    access: ModelAccess,
    model: &str,
    field: &str,
    value_type: ValueType,
    sensitivity: DataSensitivity,
) -> Result<(), CompileError> {
    if matches!(value_type, ValueType::Credential(_)) {
        return Err(CompileError::security(
            "SEC-A04-006",
            format!("credential field `{model}.{field}` cannot be exposed through a generic public projection"),
            Some("use a dedicated credential protocol primitive rather than exposing password/token/key material as model data".into()),
        ));
    }
    match sensitivity {
        DataSensitivity::Public | DataSensitivity::Redacted => Ok(()),
        DataSensitivity::Sensitive if access == ModelAccess::Authorized => Ok(()),
        DataSensitivity::Sensitive => Err(CompileError::security(
            "SEC-A01-012",
            format!(
                "Sensitive field `{model}.{field}` cannot be exposed without object authorization"
            ),
            Some("authorize the loaded object before exposing this field".into()),
        )),
        DataSensitivity::Secret => Err(CompileError::security(
            "SEC-DATA-005",
            format!("Secret field `{model}.{field}` can never be exposed in a response"),
            Some("derive a deliberately public value instead of exposing the secret".into()),
        )),
    }
}
