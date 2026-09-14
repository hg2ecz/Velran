use crate::diagnostics::CompileError;
use crate::module_namespace::resolve;
use language_core::{FormField, Program, ValidationRule};

pub(super) fn query_fields(
    route_name: &str,
    token: &str,
    namespace: &str,
    program: &Program,
) -> Result<Vec<FormField>, CompileError> {
    let schema_name = resolve(namespace, token);
    let schema = program.json_schema(&schema_name).ok_or_else(|| {
        CompileError::Syntax(format!(
            "route `{route_name}` references unknown query struct `{token}`"
        ))
    })?;
    if schema.fields.iter().any(|f| {
        matches!(
            f.ty,
            language_core::ValueType::Upload | language_core::ValueType::Image
        )
    }) {
        return Err(CompileError::Syntax(format!(
            "route `{route_name}` query struct `{token}` cannot contain Upload/Image fields"
        )));
    }
    Ok(schema.fields.clone())
}

pub(super) fn form_fields(
    route_name: &str,
    token: &str,
    namespace: &str,
    program: &Program,
) -> Result<(Vec<FormField>, Vec<ValidationRule>, Option<String>), CompileError> {
    let schema_name = resolve(namespace, token);
    let legacy = program.form(&schema_name);
    let typed = program.json_schema(&schema_name);
    match (legacy, typed) {
        (Some(_), Some(_)) => Err(CompileError::Syntax(format!(
            "route `{route_name}` form schema `{token}` is ambiguous between legacy form and Rust struct"
        ))),
        (Some(schema), None) => Ok((
            schema.fields.clone(),
            schema.validations.clone(),
            Some(schema.name.clone()),
        )),
        (None, Some(schema)) => {
            if schema.fields.iter().any(|f| {
                matches!(
                    f.ty,
                    language_core::ValueType::Upload | language_core::ValueType::Image
                )
            }) {
                return Err(CompileError::Syntax(format!(
                    "route `{route_name}` form struct `{token}` cannot contain Upload/Image fields"
                )));
            }
            Ok((schema.fields.clone(), Vec::new(), None))
        }
        (None, None) => Err(CompileError::Syntax(format!(
            "route `{route_name}` references unknown form struct `{token}`"
        ))),
    }
}

pub(super) fn multipart_fields(
    route_name: &str,
    token: &str,
    namespace: &str,
    program: &Program,
) -> Result<(Vec<FormField>, language_core::UploadField, String), CompileError> {
    let schema_name = resolve(namespace, token);
    let schema = program.json_schema(&schema_name).ok_or_else(|| {
        CompileError::Syntax(format!(
            "route `{route_name}` references unknown multipart struct `{token}`"
        ))
    })?;
    let mut upload: Option<language_core::UploadField> = None;
    for field in &schema.fields {
        match field.ty {
            language_core::ValueType::Upload | language_core::ValueType::Image => {
                if upload.is_some() {
                    return Err(CompileError::Syntax(format!(
                        "route `{route_name}` multipart struct `{token}` must contain exactly one Upload/Image field"
                    )));
                }
                upload = Some(language_core::UploadField {
                    name: field.name.clone(),
                    destination: String::new(),
                    image: field.ty == language_core::ValueType::Image,
                    publish: false,
                });
            }
            language_core::ValueType::String
            | language_core::ValueType::Email
            | language_core::ValueType::Url
            | language_core::ValueType::Slug
            | language_core::ValueType::Int
            | language_core::ValueType::Bool => {}
            _ => {
                return Err(CompileError::Syntax(format!(
                    "route `{route_name}` multipart field `{}` has unsupported type; v1 allows String/Email/Url/Slug/i64/bool plus exactly one Upload/Image",
                    field.name
                )));
            }
        }
    }
    let upload = upload.ok_or_else(|| CompileError::Syntax(format!(
        "route `{route_name}` multipart struct `{token}` must contain exactly one Upload/Image field"
    )))?;
    Ok((schema.fields.clone(), upload, schema_name))
}
