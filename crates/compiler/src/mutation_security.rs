use crate::diagnostics::CompileError;
use crate::expression_security::infer_static_expr_type;
use crate::handler_types::StaticType;
use crate::scalar_security::{LifecycleEvidence, TrustLevel};
use language_core::{CredentialLifecycleMode, CredentialPurpose, Expr, Program, QueryFunction};
use std::collections::HashMap;

pub(super) fn validate_mutation_call(
    query: &QueryFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    if query.credential_lifecycle.is_some() {
        return validate_lifecycle_call(query, args, known, program);
    }
    let Some(target) = &query.mutation_target else {
        return Ok(());
    };
    let parameter_index = query
        .params
        .iter()
        .position(|param| param.name == target.key)
        .ok_or_else(|| {
            CompileError::Syntax(format!(
                "query `{}` mutation key `{}` is not a parameter",
                query.name, target.key
            ))
        })?;
    let argument = args.get(parameter_index).ok_or_else(|| {
        CompileError::Syntax(format!(
            "query `{}` mutation key argument is missing",
            query.name
        ))
    })?;
    let evidence = infer_static_expr_type(argument, known, program)?
        .scalar()
        .and_then(|scalar| scalar.mutation);
    if let Some(proof) = evidence {
        if proof.model == target.model && proof.key == target.key {
            return Ok(());
        }
    }

    Err(CompileError::security(
        "SEC-A01-005",
        format!(
            "mutation query `{}` requires authorization proof for `{}.{}`",
            query.name, target.model, target.key
        ),
        Some(format!(
            "load and authorize the `{}` object, then pass its `{}` field (or an unchanged local alias) to `{}`",
            target.model, target.key, query.name
        )),
    ))
}

fn validate_lifecycle_call(
    query: &QueryFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    let target = query.credential_lifecycle.as_ref().ok_or_else(|| {
        CompileError::Syntax("internal: missing credential lifecycle target".into())
    })?;
    let parameter_index = query
        .params
        .iter()
        .position(|param| param.name == target.hash_field)
        .ok_or_else(|| {
            CompileError::Syntax(format!(
                "query `{}` lifecycle hash parameter `{}` is missing",
                query.name, target.hash_field
            ))
        })?;
    let argument = args.get(parameter_index).ok_or_else(|| {
        CompileError::Syntax(format!(
            "query `{}` lifecycle hash argument is missing",
            query.name
        ))
    })?;
    let scalar = infer_static_expr_type(argument, known, program)?
        .scalar()
        .ok_or_else(|| {
            CompileError::Syntax(format!(
                "query `{}` lifecycle hash argument must be scalar",
                query.name
            ))
        })?;
    if scalar.lifecycle != Some(LifecycleEvidence::PresentedTokenHash(target.hash_purpose)) {
        return Err(CompileError::security(
            "SEC-A07-015",
            format!(
                "credential lifecycle query `{}` requires presented-token proof for `{}`",
                query.name, target.hash_field
            ),
            Some(format!(
                "derive `{}` from the validated presented token with `presentedTokenHash(token)` and pass that unchanged value to `{}`",
                target.hash_field, query.name
            )),
        ));
    }
    if target.mode == CredentialLifecycleMode::RotateSession {
        validate_rotation_replacement(query, target, args, known, program)?;
    }
    Ok(())
}

fn validate_rotation_replacement(
    query: &QueryFunction,
    target: &language_core::CredentialLifecycleTarget,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    let replacement = target.replacement_hash_param.as_ref().ok_or_else(|| {
        CompileError::Syntax(
            "internal: rotation contract missing replacement hash parameter".into(),
        )
    })?;
    let index = query
        .params
        .iter()
        .position(|param| &param.name == replacement)
        .ok_or_else(|| {
            CompileError::Syntax(format!(
                "query `{}` replacement hash parameter `{replacement}` is missing",
                query.name
            ))
        })?;
    let argument = args.get(index).ok_or_else(|| {
        CompileError::Syntax(format!(
            "query `{}` replacement hash argument is missing",
            query.name
        ))
    })?;
    let lifecycle = infer_static_expr_type(argument, known, program)?
        .scalar()
        .and_then(|scalar| scalar.lifecycle);
    if lifecycle
        == Some(LifecycleEvidence::IssuedTokenHash(
            CredentialPurpose::SessionTokenHash,
        ))
    {
        return Ok(());
    }
    Err(CompileError::security(
        "SEC-A07-018",
        format!(
            "session rotation query `{}` requires a freshly issued replacement token hash",
            query.name
        ),
        Some(format!(
            "derive `{replacement}` from a fresh token with `tokenHash(newSessionToken())` and pass it unchanged to `{}`",
            query.name
        )),
    ))
}

pub(super) fn validate_mutation_inputs(
    query: &QueryFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    for (param, argument) in query.params.iter().zip(args) {
        let Some(scalar) = infer_static_expr_type(argument, known, program)?.scalar() else {
            continue;
        };
        if scalar.trust == TrustLevel::Untrusted {
            return Err(CompileError::security(
                "SEC-DATA-003",
                format!(
                    "mutation query `{}` receives unvalidated external data for `{}`",
                    query.name, param.name
                ),
                Some(format!(
                    "validate `{}` at the route boundary before passing it to `{}`",
                    param.name, query.name
                )),
            ));
        }
    }
    Ok(())
}
