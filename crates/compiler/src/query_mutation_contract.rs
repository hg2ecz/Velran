use crate::diagnostics::CompileError;
use crate::module_namespace::resolve;
use language_core::{
    CredentialLifecycleMode, CredentialLifecycleTarget, CredentialPurpose, DataSensitivity,
    FunctionParam, MutationTarget, Program, QueryReturn, ValueType,
};

pub(super) struct QueryContracts {
    pub return_type: String,
    pub mutation_target: Option<MutationTarget>,
    pub credential_lifecycle: Option<CredentialLifecycleTarget>,
}

pub(super) fn parse_query_contracts(
    declaration: &str,
    namespace: &str,
    program: &Program,
    params: &[FunctionParam],
    query_name: &str,
) -> Result<QueryContracts, CompileError> {
    if declaration.contains(" mutates ")
        && (declaration.contains(" consumes ")
            || declaration.contains(" revokes ")
            || declaration.contains(" rotates "))
    {
        return Err(CompileError::Syntax(format!(
            "query `{query_name}` cannot combine object mutation and credential lifecycle contracts"
        )));
    }
    if let Some((return_part, contract_part)) = declaration.rsplit_once(" mutates ") {
        return Ok(QueryContracts {
            return_type: return_part.trim().to_string(),
            mutation_target: Some(parse_mutation_target(
                contract_part.trim(),
                namespace,
                program,
                params,
                query_name,
            )?),
            credential_lifecycle: None,
        });
    }
    for (keyword, mode) in [
        (" consumes ", CredentialLifecycleMode::ConsumeReset),
        (" revokes ", CredentialLifecycleMode::RevokeSession),
        (" rotates ", CredentialLifecycleMode::RotateSession),
    ] {
        if let Some((return_part, contract_part)) = declaration.rsplit_once(keyword) {
            return Ok(QueryContracts {
                return_type: return_part.trim().to_string(),
                mutation_target: None,
                credential_lifecycle: Some(parse_credential_lifecycle_target(
                    contract_part.trim(),
                    namespace,
                    program,
                    params,
                    query_name,
                    mode,
                )?),
            });
        }
    }
    Ok(QueryContracts {
        return_type: declaration.trim().to_string(),
        mutation_target: None,
        credential_lifecycle: None,
    })
}

pub(super) fn validate_sql_mutation_contract(
    query_name: &str,
    sql_keyword: &str,
    mutation_target: &Option<MutationTarget>,
    lifecycle: &Option<CredentialLifecycleTarget>,
    return_type: &QueryReturn,
    sql: &str,
) -> Result<(), CompileError> {
    if let Some(target) = lifecycle {
        let expected_keyword = match target.mode {
            CredentialLifecycleMode::ConsumeReset | CredentialLifecycleMode::RevokeSession => {
                "DELETE"
            }
            CredentialLifecycleMode::RotateSession => "UPDATE",
        };
        if sql_keyword != expected_keyword {
            return Err(CompileError::security(
                "SEC-A07-007",
                format!(
                    "credential lifecycle query `{query_name}` must use {expected_keyword} for its atomic state transition"
                ),
                Some("consume/revoke with DELETE; rotate sessions with one guarded UPDATE that replaces hash and expiry".into()),
            ));
        }
        if !matches!(return_type, QueryReturn::Changed) {
            return Err(CompileError::security(
                "SEC-A07-008",
                format!("credential lifecycle query `{query_name}` must return Changed"),
                Some("Changed enforces exactly one affected row, making token consumption/revocation atomic and replay-safe".into()),
            ));
        }
        validate_lifecycle_sql_predicates(query_name, target, sql)?;
        return Ok(());
    }

    match sql_keyword {
        "UPDATE" | "DELETE" if mutation_target.is_none() => Err(CompileError::security(
            "SEC-A01-006",
            format!("mutation query `{query_name}` must declare its authorization target"),
            Some("add `mutates <Model> by <key>` before the `sql` block".to_string()),
        )),
        "INSERT" if mutation_target.is_some() => Err(CompileError::Syntax(format!(
            "insert query `{query_name}` cannot declare `mutates`; authorization proof applies to existing objects"
        ))),
        _ if mutation_target.is_some() && !matches!(sql_keyword, "UPDATE" | "DELETE") => {
            Err(CompileError::Syntax(format!(
                "query `{query_name}` declares `mutates` but is not UPDATE or DELETE"
            )))
        }
        _ => Ok(()),
    }
}

fn parse_mutation_target(
    text: &str,
    namespace: &str,
    program: &Program,
    params: &[FunctionParam],
    query_name: &str,
) -> Result<MutationTarget, CompileError> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() != 3 || words[1] != "by" {
        return Err(CompileError::Syntax(format!(
            "query `{query_name}` mutation contract syntax is `mutates <Model> by <key>`"
        )));
    }
    let model_name = resolve(namespace, words[0]);
    let key = words[2];
    let model = program
        .model(&model_name)
        .ok_or_else(|| CompileError::UnknownModel(model_name.clone()))?;
    let field = model
        .fields
        .iter()
        .find(|field| field.name == key)
        .ok_or_else(|| {
            CompileError::Syntax(format!(
                "query `{query_name}` mutation key `{key}` does not exist on model `{model_name}`"
            ))
        })?;
    let param = params
        .iter()
        .find(|param| param.name == key)
        .ok_or_else(|| {
            CompileError::Syntax(format!(
                "query `{query_name}` mutation key `{key}` must also be a query parameter"
            ))
        })?;
    if param.ty != field.ty || !mutation_key_type_allowed(param.ty) {
        return Err(CompileError::Syntax(format!(
            "query `{query_name}` mutation key `{key}` must have the same scalar type as `{model_name}.{key}`"
        )));
    }
    Ok(MutationTarget {
        model: model_name,
        key: key.to_string(),
    })
}

fn parse_credential_lifecycle_target(
    text: &str,
    namespace: &str,
    program: &Program,
    params: &[FunctionParam],
    query_name: &str,
    mode: CredentialLifecycleMode,
) -> Result<CredentialLifecycleTarget, CompileError> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let (model_raw, hash_field, expiry_field, replacement_hash_param) = match mode {
        CredentialLifecycleMode::ConsumeReset
            if words.len() == 5 && words[1] == "by" && words[3] == "before" =>
        {
            (words[0], words[2], Some(words[4]), None)
        }
        CredentialLifecycleMode::RevokeSession if words.len() == 3 && words[1] == "by" => {
            (words[0], words[2], None, None)
        }
        CredentialLifecycleMode::RotateSession
            if words.len() == 7 && words[1] == "by" && words[3] == "to" && words[5] == "until" =>
        {
            (words[0], words[2], Some(words[6]), Some(words[4]))
        }
        _ => {
            let syntax = match mode {
                CredentialLifecycleMode::ConsumeReset => {
                    "consumes <Model> by <tokenHashField> before <expiresAtField>"
                }
                CredentialLifecycleMode::RevokeSession => "revokes <Model> by <tokenHashField>",
                CredentialLifecycleMode::RotateSession => {
                    "rotates <Model> by <tokenHashField> to <newTokenHashParam> until <expiresAtField>"
                }
            };
            return Err(CompileError::Syntax(format!(
                "query `{query_name}` lifecycle contract syntax is `{syntax}`"
            )));
        }
    };
    let model_name = resolve(namespace, model_raw);
    let model = program
        .model(&model_name)
        .ok_or_else(|| CompileError::UnknownModel(model_name.clone()))?;
    let hash_model_field = model.fields.iter().find(|field| field.name == hash_field).ok_or_else(|| {
        CompileError::Syntax(format!("query `{query_name}` lifecycle hash field `{hash_field}` does not exist on model `{model_name}`"))
    })?;
    let expected_purpose = match mode {
        CredentialLifecycleMode::ConsumeReset => CredentialPurpose::PasswordResetTokenHash,
        CredentialLifecycleMode::RevokeSession | CredentialLifecycleMode::RotateSession => {
            CredentialPurpose::SessionTokenHash
        }
    };
    if hash_model_field.ty != ValueType::Credential(expected_purpose)
        || hash_model_field.sensitivity != DataSensitivity::Secret
    {
        return Err(CompileError::security(
            "SEC-A07-009",
            format!(
                "{model_name}.{hash_field} must be Secret<{}>",
                expected_purpose.source_name()
            ),
            Some("persist only purpose-matched token hashes in lifecycle state".into()),
        ));
    }
    if let Some(expiry_field) = expiry_field {
        let field = model.fields.iter().find(|field| field.name == expiry_field).ok_or_else(|| {
            CompileError::Syntax(format!("query `{query_name}` lifecycle expiry field `{expiry_field}` does not exist on model `{model_name}`"))
        })?;
        if field.ty != ValueType::DateTime {
            return Err(CompileError::security(
                "SEC-A07-010",
                format!("{model_name}.{expiry_field} must be DateTime"),
                Some("store an explicit expiry timestamp for credential lifecycle state".into()),
            ));
        }
        if mode == CredentialLifecycleMode::RotateSession {
            let param = params.iter().find(|param| param.name == expiry_field).ok_or_else(|| {
                CompileError::Syntax(format!(
                    "query `{query_name}` rotation expiry `{expiry_field}` must also be a query parameter"
                ))
            })?;
            if param.ty != ValueType::DateTime {
                return Err(CompileError::security(
                    "SEC-A07-017",
                    format!("query `{query_name}` rotation expiry `{expiry_field}` must be DateTime"),
                    Some("pass the new session expiry explicitly and update it atomically with the token hash".into()),
                ));
            }
        }
    }
    let hash_param = params.iter().find(|param| param.name == hash_field).ok_or_else(|| {
        CompileError::Syntax(format!("query `{query_name}` lifecycle hash field `{hash_field}` must also be a query parameter"))
    })?;
    if hash_param.ty != ValueType::Credential(expected_purpose)
        || hash_param.sensitivity != DataSensitivity::Public
    {
        return Err(CompileError::security(
            "SEC-A07-011",
            format!(
                "query `{query_name}` `{hash_field}` parameter must be a presented {} value",
                expected_purpose.source_name()
            ),
            Some("derive it from the presented bearer token with presentedTokenHash(token)".into()),
        ));
    }
    if let Some(replacement_param) = replacement_hash_param {
        let param = params.iter().find(|param| param.name == replacement_param).ok_or_else(|| {
            CompileError::Syntax(format!(
                "query `{query_name}` replacement hash parameter `{replacement_param}` is missing"
            ))
        })?;
        if param.ty != ValueType::Credential(CredentialPurpose::SessionTokenHash)
            || param.sensitivity != DataSensitivity::Secret
        {
            return Err(CompileError::security(
                "SEC-A07-016",
                format!(
                    "query `{query_name}` replacement hash `{replacement_param}` must be Secret<SessionTokenHash>"
                ),
                Some("derive the replacement hash from a freshly issued session token with tokenHash(newSessionToken())".into()),
            ));
        }
    }
    Ok(CredentialLifecycleTarget {
        mode,
        model: model_name,
        hash_field: hash_field.to_string(),
        expiry_field: expiry_field.map(str::to_string),
        hash_purpose: expected_purpose,
        replacement_hash_param: replacement_hash_param.map(str::to_string),
    })
}

fn validate_lifecycle_sql_predicates(
    query_name: &str,
    target: &CredentialLifecycleTarget,
    sql: &str,
) -> Result<(), CompileError> {
    let normalized = sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let (mutation_clause, where_clause) = normalized
        .split_once(" where ")
        .unwrap_or((normalized.as_str(), ""));
    let hash_bind = format!(":{}", target.hash_field).to_ascii_lowercase();
    let hash_is_equality = where_clause.contains(&format!("= {hash_bind}"))
        || where_clause.contains(&format!("{hash_bind} ="));
    let expiry_is_guarded = match target.mode {
        CredentialLifecycleMode::ConsumeReset | CredentialLifecycleMode::RotateSession => {
            where_clause.contains("> current_timestamp")
                || where_clause.contains(">= current_timestamp")
                || where_clause.contains("current_timestamp <")
                || where_clause.contains("current_timestamp <=")
        }
        CredentialLifecycleMode::RevokeSession => true,
    };
    let rotation_assignments_are_present = match target.mode {
        CredentialLifecycleMode::RotateSession => {
            let replacement = target
                .replacement_hash_param
                .as_ref()
                .expect("rotation contract has replacement hash parameter");
            let expiry = target
                .expiry_field
                .as_ref()
                .expect("rotation contract has expiry field");
            let replacement_bind = format!(":{replacement}").to_ascii_lowercase();
            let expiry_bind = format!(":{expiry}").to_ascii_lowercase();
            mutation_clause.contains(&format!("= {replacement_bind}"))
                && mutation_clause.contains(&format!("= {expiry_bind}"))
        }
        _ => true,
    };
    if !hash_is_equality || !expiry_is_guarded || !rotation_assignments_are_present {
        let help = match target.mode {
            CredentialLifecycleMode::ConsumeReset => format!(
                "DELETE the reset row with equality on `:{}` and an expiry predicate against CURRENT_TIMESTAMP",
                target.hash_field
            ),
            CredentialLifecycleMode::RevokeSession => format!(
                "DELETE the session row with equality on `:{}`",
                target.hash_field
            ),
            CredentialLifecycleMode::RotateSession => format!(
                "UPDATE the session row with equality on `:{}`, replace the token hash, set the new expiry, and guard the old expiry with CURRENT_TIMESTAMP",
                target.hash_field
            ),
        };
        return Err(CompileError::security(
            "SEC-A07-013",
            format!(
                "credential lifecycle query `{query_name}` is missing its atomic lifecycle predicate"
            ),
            Some(help),
        ));
    }
    Ok(())
}

fn mutation_key_type_allowed(value_type: ValueType) -> bool {
    !matches!(
        value_type,
        ValueType::Upload
            | ValueType::Image
            | ValueType::F32Array
            | ValueType::StringList
            | ValueType::StringDict
    )
}
