use crate::diagnostics::CompileError;
use crate::expression::infer_expr_type;
use crate::handler_types::StaticType;
use crate::model_security::ModelAccess;
use crate::scalar_security::{DisclosureEvidence, ScalarType, TrustLevel};
use language_core::{DataSensitivity, Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) fn infer_static_expr_type(
    e: &Expr,
    k: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<StaticType, CompileError> {
    match e {
        Expr::Variable(name) => k
            .get(name)
            .cloned()
            .ok_or_else(|| CompileError::UnknownVariable(name.clone())),
        Expr::Field { base, field } => infer_field_static_type(base, field, k, p),
        _ => {
            let result_type = infer_expr_type(e, k, p)?;
            let metadata = expression_metadata(e, k, p)?;
            Ok(StaticType::Scalar(ScalarType {
                value_type: result_type,
                trust: metadata.trust,
                sensitivity: metadata.sensitivity,
                disclosure: metadata.disclosure,
                mutation: metadata.mutation,
                tenant: metadata.tenant,
                lifecycle: metadata.lifecycle,
            }))
        }
    }
}

fn infer_field_static_type(
    base: &str,
    field: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<StaticType, CompileError> {
    match known.get(base) {
        Some(StaticType::Model(model_type)) => {
            let model = program
                .model(&model_type.name)
                .ok_or_else(|| CompileError::UnknownModel(model_type.name.clone()))?;
            let field = model
                .fields
                .iter()
                .find(|candidate| candidate.name == field)
                .ok_or_else(|| CompileError::Syntax(format!("unknown field `{base}.{field}`")))?;
            let scalar = match model_type.access {
                ModelAccess::Authorized => ScalarType::authorized_field(
                    field.ty,
                    field.sensitivity,
                    model_type.name.clone(),
                    field.name.clone(),
                ),
                ModelAccess::Unverified => ScalarType::classified(field.ty, field.sensitivity),
            };
            Ok(StaticType::Scalar(scalar))
        }
        Some(StaticType::PureStruct(schema)) => {
            let def = program
                .json_schema(schema)
                .ok_or_else(|| CompileError::Syntax(format!("unknown pure struct `{schema}`")))?;
            let field = def
                .fields
                .iter()
                .find(|candidate| candidate.name == field)
                .ok_or_else(|| CompileError::Syntax(format!("unknown field `{base}.{field}`")))?;
            Ok(StaticType::trusted_scalar(field.ty))
        }
        Some(StaticType::Upload) => {
            let value_type = match field {
                "path" | "filename" | "contentType" => ValueType::String,
                "bytes" => ValueType::Int,
                _ => {
                    return Err(CompileError::Syntax(format!(
                        "Upload has no field `{field}`"
                    )));
                }
            };
            Ok(StaticType::untrusted_scalar(value_type))
        }
        Some(value) if value.is_scalar(ValueType::Image) => {
            let value_type = match field {
                "path" | "contentType" => ValueType::String,
                "width" | "height" | "bytes" => ValueType::Int,
                _ => {
                    return Err(CompileError::Syntax(format!(
                        "Image has no field `{field}`"
                    )));
                }
            };
            Ok(StaticType::untrusted_scalar(value_type))
        }
        _ => Err(CompileError::Syntax(format!(
            "`{base}` is not a model/upload/pure struct"
        ))),
    }
}

fn expression_metadata(
    e: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ScalarType, CompileError> {
    let trusted_public = || ScalarType::trusted(ValueType::Bool);
    match e {
        Expr::Variable(name) => known
            .get(name)
            .and_then(StaticType::scalar)
            .ok_or_else(|| CompileError::UnknownVariable(name.clone())),
        Expr::Field { base, field } => infer_field_static_type(base, field, known, program)?
            .scalar()
            .ok_or_else(|| CompileError::Syntax(format!("`{base}.{field}` is not scalar"))),
        Expr::PureSumPredicate { .. } => Ok(ScalarType::untrusted(ValueType::Bool)),
        Expr::PureSumUnwrapOr { fallback, .. } => {
            let ty = infer_expr_type(e, known, program)?;
            let fallback_meta = expression_metadata(fallback, known, program)?;
            Ok(ScalarType {
                value_type: ty,
                ..fallback_meta
            })
        }
        Expr::Slugify(inner) | Expr::Not(inner) => expression_metadata(inner, known, program),
        Expr::Builtin { function, args } => match function {
            language_core::BuiltinFunction::PasswordHash => {
                expression_metadata(&args[0], known, program)?;
                Ok(ScalarType {
                    value_type: ValueType::Credential(
                        language_core::CredentialPurpose::PasswordHash,
                    ),
                    trust: TrustLevel::Trusted,
                    sensitivity: DataSensitivity::Secret,
                    disclosure: DisclosureEvidence::None,
                    mutation: None,
                    tenant: None,
                    lifecycle: None,
                })
            }
            language_core::BuiltinFunction::PasswordVerify
            | language_core::BuiltinFunction::TokenMatches
            | language_core::BuiltinFunction::TokenActive
            | language_core::BuiltinFunction::VerifyWebhookSignature => {
                Ok(ScalarType::trusted(ValueType::Bool))
            }
            language_core::BuiltinFunction::SignWebhook => {
                Ok(ScalarType::trusted(ValueType::String))
            }
            language_core::BuiltinFunction::EncryptUserData
            | language_core::BuiltinFunction::DecryptUserData => Ok(ScalarType {
                value_type: ValueType::String,
                trust: TrustLevel::Trusted,
                sensitivity: DataSensitivity::Sensitive,
                disclosure: DisclosureEvidence::None,
                mutation: None,
                tenant: None,
                lifecycle: None,
            }),
            language_core::BuiltinFunction::Redact => {
                expression_metadata(&args[0], known, program)?;
                Ok(ScalarType {
                    value_type: ValueType::String,
                    trust: TrustLevel::Trusted,
                    sensitivity: DataSensitivity::Redacted,
                    disclosure: DisclosureEvidence::None,
                    mutation: None,
                    tenant: None,
                    lifecycle: None,
                })
            }
            language_core::BuiltinFunction::NewSessionToken => Ok(ScalarType::issued_token(
                ValueType::Credential(language_core::CredentialPurpose::SessionToken),
                language_core::CredentialPurpose::SessionToken,
            )),
            language_core::BuiltinFunction::NewPasswordResetToken => Ok(ScalarType::issued_token(
                ValueType::Credential(language_core::CredentialPurpose::PasswordResetToken),
                language_core::CredentialPurpose::PasswordResetToken,
            )),
            language_core::BuiltinFunction::NewCsrfToken => Ok(ScalarType::issued_token(
                ValueType::Credential(language_core::CredentialPurpose::CsrfToken),
                language_core::CredentialPurpose::CsrfToken,
            )),
            language_core::BuiltinFunction::TokenHash => {
                let source = expression_metadata(&args[0], known, program)?;
                let Some(purpose) = token_hash_purpose(source.value_type) else {
                    return Ok(source);
                };
                Ok(ScalarType::issued_token_hash(
                    ValueType::Credential(purpose),
                    purpose,
                ))
            }
            language_core::BuiltinFunction::PresentedTokenHash => {
                let source = expression_metadata(&args[0], known, program)?;
                let Some(purpose) = token_hash_purpose(source.value_type) else {
                    return Ok(source);
                };
                Ok(ScalarType::presented_token_hash(
                    ValueType::Credential(purpose),
                    purpose,
                ))
            }
            language_core::BuiltinFunction::SafeHtmlEmpty
            | language_core::BuiltinFunction::SafeHtmlText
            | language_core::BuiltinFunction::SafeHtmlElement
            | language_core::BuiltinFunction::SafeHtmlLink
            | language_core::BuiltinFunction::SafeHtmlConcat => {
                let mut metadata = combine_expression_metadata(args.iter(), known, program)?;
                metadata.value_type = ValueType::Domain(language_core::SAFE_HTML_DOMAIN_ID);
                metadata.trust = TrustLevel::Trusted;
                Ok(metadata)
            }
            _ => combine_expression_metadata(args.iter(), known, program),
        },
        Expr::Binary { left, right, .. } => {
            let left = expression_metadata(left, known, program)?;
            let right = expression_metadata(right, known, program)?;
            Ok(left.combine(right, ValueType::Bool))
        }
        Expr::F32ArrayNew { len, fill } => {
            let len = expression_metadata(len, known, program)?;
            let fill = expression_metadata(fill, known, program)?;
            Ok(len.combine(fill, ValueType::Bool))
        }
        Expr::CollectionIndex { collection, index } => {
            let collection = known
                .get(collection)
                .and_then(StaticType::scalar)
                .unwrap_or_else(trusted_public);
            let index = expression_metadata(index, known, program)?;
            Ok(collection.combine(index, ValueType::Bool))
        }
        Expr::CollectionLen { collection } => Ok(known
            .get(collection)
            .and_then(StaticType::scalar)
            .unwrap_or_else(trusted_public)),
        Expr::String(_)
        | Expr::Int(_)
        | Expr::F32(_)
        | Expr::Bool(_)
        | Expr::EnumLiteral { .. } => Ok(trusted_public()),
    }
}

fn combine_expression_metadata<'a>(
    expressions: impl Iterator<Item = &'a Expr>,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ScalarType, CompileError> {
    let mut combined = ScalarType::trusted(ValueType::Bool);
    for expression in expressions {
        let metadata = expression_metadata(expression, known, program)?;
        combined = combined.combine(metadata, ValueType::Bool);
    }
    Ok(combined)
}

fn token_hash_purpose(value_type: ValueType) -> Option<language_core::CredentialPurpose> {
    match value_type {
        ValueType::Credential(language_core::CredentialPurpose::SessionToken) => {
            Some(language_core::CredentialPurpose::SessionTokenHash)
        }
        ValueType::Credential(language_core::CredentialPurpose::PasswordResetToken) => {
            Some(language_core::CredentialPurpose::PasswordResetTokenHash)
        }
        ValueType::Credential(language_core::CredentialPurpose::CsrfToken) => {
            Some(language_core::CredentialPurpose::CsrfTokenHash)
        }
        _ => None,
    }
}
