use crate::diagnostics::CompileError;
use crate::expression_security::infer_static_expr_type;
use crate::handler_types::StaticType;
use crate::scalar_security::TrustLevel;
use language_core::{
    BuiltinFunction, CredentialPurpose, DataSensitivity, Expr, Program, ValueType,
};
use std::collections::HashMap;

pub(super) fn handles(function: BuiltinFunction) -> bool {
    matches!(
        function,
        BuiltinFunction::PasswordHash
            | BuiltinFunction::PasswordVerify
            | BuiltinFunction::NewSessionToken
            | BuiltinFunction::NewPasswordResetToken
            | BuiltinFunction::NewCsrfToken
            | BuiltinFunction::TokenHash
            | BuiltinFunction::PresentedTokenHash
            | BuiltinFunction::TokenMatches
            | BuiltinFunction::TokenActive
            | BuiltinFunction::SignWebhook
            | BuiltinFunction::VerifyWebhookSignature
            | BuiltinFunction::EncryptUserData
            | BuiltinFunction::DecryptUserData
    )
}

pub(super) fn infer(
    function: BuiltinFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ValueType, CompileError> {
    match function {
        BuiltinFunction::PasswordHash => {
            require_password(&args[0], known, program, "passwordHash")?;
            Ok(ValueType::Credential(CredentialPurpose::PasswordHash))
        }
        BuiltinFunction::PasswordVerify => {
            require_password_hash(&args[0], known, program)?;
            require_password(&args[1], known, program, "passwordVerify")?;
            Ok(ValueType::Bool)
        }
        BuiltinFunction::NewSessionToken => Ok(token_type(CredentialPurpose::SessionToken)),
        BuiltinFunction::NewPasswordResetToken => {
            Ok(token_type(CredentialPurpose::PasswordResetToken))
        }
        BuiltinFunction::NewCsrfToken => Ok(token_type(CredentialPurpose::CsrfToken)),
        BuiltinFunction::TokenHash => token_hash_type(&args[0], known, program),
        BuiltinFunction::PresentedTokenHash => presented_token_hash_type(&args[0], known, program),
        BuiltinFunction::TokenMatches => {
            require_csrf_token_pair(&args[0], &args[1], known, program)?;
            Ok(ValueType::Bool)
        }
        BuiltinFunction::TokenActive => {
            require_active_token(&args[0], &args[1], &args[2], known, program)?;
            Ok(ValueType::Bool)
        }
        BuiltinFunction::SignWebhook => {
            require_crypto_key(
                &args[0],
                known,
                program,
                CredentialPurpose::SigningKeyWebhook,
                "signWebhook",
            )?;
            require_public_string(&args[1], known, program, "signWebhook payload")?;
            Ok(ValueType::String)
        }
        BuiltinFunction::VerifyWebhookSignature => {
            require_verification_key(&args[0], known, program)?;
            require_public_string(&args[1], known, program, "verifyWebhookSignature payload")?;
            require_public_string(&args[2], known, program, "verifyWebhookSignature signature")?;
            Ok(ValueType::Bool)
        }
        BuiltinFunction::EncryptUserData => {
            require_crypto_key(
                &args[0],
                known,
                program,
                CredentialPurpose::EncryptionKeyUserData,
                "encryptUserData",
            )?;
            require_sensitive_string(&args[1], known, program, "encryptUserData plaintext")?;
            Ok(ValueType::String)
        }
        BuiltinFunction::DecryptUserData => {
            require_decryption_key(&args[0], known, program)?;
            require_sensitive_string(&args[1], known, program, "decryptUserData ciphertext")?;
            Ok(ValueType::String)
        }
        _ => Err(CompileError::Syntax(
            "internal: non-credential builtin routed to credential checker".into(),
        )),
    }
}

fn require_password(
    expr: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
    function: &str,
) -> Result<(), CompileError> {
    let scalar = scalar(expr, known, program, function)?;
    if scalar.value_type != ValueType::Credential(CredentialPurpose::Password) {
        return Err(CompileError::security(
            "SEC-A04-003",
            format!("{function} requires Password, got a different value type"),
            Some("declare password inputs with the built-in `Password` type; do not pass arbitrary String values to credential primitives".into()),
        ));
    }
    if scalar.trust == TrustLevel::Untrusted {
        return Err(CompileError::security(
            "SEC-A04-002",
            format!("{function} requires validated Password input"),
            Some("bind the password through a typed route/form parameter before using credential primitives".into()),
        ));
    }
    Ok(())
}

fn require_password_hash(
    expr: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    let scalar = scalar(expr, known, program, "passwordVerify")?;
    if scalar.value_type != ValueType::Credential(CredentialPurpose::PasswordHash)
        || scalar.sensitivity != DataSensitivity::Secret
    {
        return Err(CompileError::security(
            "SEC-A04-001",
            "passwordVerify hash must be Secret<PasswordHash>",
            Some(
                "store password hashes in `Secret<PasswordHash>` fields or query parameters".into(),
            ),
        ));
    }
    Ok(())
}

fn token_type(purpose: CredentialPurpose) -> ValueType {
    ValueType::Credential(purpose)
}

fn token_hash_type(
    token: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ValueType, CompileError> {
    let token = scalar(token, known, program, "tokenHash")?;
    let Some(raw_purpose) = raw_token_purpose(token.value_type) else {
        return Err(token_hash_contract_error());
    };
    if token.sensitivity != DataSensitivity::Secret
        || token.lifecycle
            != Some(crate::scalar_security::LifecycleEvidence::IssuedToken(
                raw_purpose,
            ))
    {
        return Err(token_hash_contract_error());
    }
    Ok(ValueType::Credential(hash_purpose(raw_purpose)))
}

fn presented_token_hash_type(
    token: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ValueType, CompileError> {
    let scalar = scalar(token, known, program, "presentedTokenHash")?;
    let purpose = match scalar.value_type {
        ValueType::Credential(CredentialPurpose::SessionToken)
        | ValueType::Credential(CredentialPurpose::PasswordResetToken)
        | ValueType::Credential(CredentialPurpose::CsrfToken)
            if scalar.trust == TrustLevel::Validated
                && scalar.sensitivity == DataSensitivity::Public =>
        {
            hash_purpose(match scalar.value_type {
                ValueType::Credential(purpose) => purpose,
                _ => unreachable!(),
            })
        }
        _ => {
            return Err(CompileError::security(
                "SEC-A07-014",
                "presentedTokenHash requires a validated presented bearer token",
                Some("use it only with SessionToken, PasswordResetToken, or CsrfToken values decoded from the request boundary".into()),
            ));
        }
    };
    Ok(ValueType::Credential(purpose))
}

fn require_matching_token_pair(
    stored: &Expr,
    presented: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
    function: &str,
) -> Result<(), CompileError> {
    let stored = scalar(stored, known, program, function)?;
    let presented = scalar(presented, known, program, function)?;
    let Some(raw_purpose) = raw_purpose_for_hash(stored.value_type) else {
        return Err(token_contract_error(&format!(
            "{function} first argument must be a stored secret token hash"
        )));
    };
    if stored.sensitivity != DataSensitivity::Secret {
        return Err(token_contract_error(&format!(
            "{function} first argument must be Secret<TokenHashPurpose>"
        )));
    }
    if presented.value_type != ValueType::Credential(raw_purpose) {
        return Err(CompileError::security(
            "SEC-A07-001",
            format!(
                "{function} requires the matching raw credential purpose; expected {}",
                raw_purpose.source_name()
            ),
            Some("do not compare session, reset, and CSRF tokens across purposes".into()),
        ));
    }
    if presented.trust == TrustLevel::Untrusted {
        return Err(CompileError::security(
            "SEC-A07-002",
            format!("{function} requires a validated presented token"),
            Some("bind the presented token through a purpose-typed request parameter before verification".into()),
        ));
    }
    Ok(())
}

fn require_csrf_token_pair(
    stored: &Expr,
    presented: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    require_matching_token_pair(stored, presented, known, program, "tokenMatches")?;
    let stored = scalar(stored, known, program, "tokenMatches")?;
    if stored.value_type != ValueType::Credential(CredentialPurpose::CsrfTokenHash) {
        return Err(CompileError::security(
            "SEC-A07-004",
            "session and password-reset tokens require expiry-aware tokenActive(...) verification",
            Some("use tokenActive(storedHash, presentedToken, expiresAt); tokenMatches(...) is reserved for CSRF tokens".into()),
        ));
    }
    Ok(())
}

fn require_active_token(
    stored: &Expr,
    presented: &Expr,
    expires_at: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    require_matching_token_pair(stored, presented, known, program, "tokenActive")?;
    let stored = scalar(stored, known, program, "tokenActive")?;
    if stored.value_type == ValueType::Credential(CredentialPurpose::CsrfTokenHash) {
        return Err(CompileError::security(
            "SEC-A07-005",
            "CSRF tokens do not use tokenActive(...) expiry verification",
            Some(
                "use tokenMatches(storedCsrfHash, presentedCsrfToken) for CSRF verification".into(),
            ),
        ));
    }
    let expires_at = scalar(expires_at, known, program, "tokenActive")?;
    if expires_at.value_type != ValueType::DateTime {
        return Err(CompileError::security(
            "SEC-A07-006",
            "tokenActive expiry argument must be DateTime",
            Some("store an explicit expiry timestamp with each session/reset token and pass that field to tokenActive(...)".into()),
        ));
    }
    Ok(())
}

fn raw_token_purpose(value_type: ValueType) -> Option<CredentialPurpose> {
    match value_type {
        ValueType::Credential(
            purpose @ (CredentialPurpose::SessionToken
            | CredentialPurpose::PasswordResetToken
            | CredentialPurpose::CsrfToken),
        ) => Some(purpose),
        _ => None,
    }
}

fn hash_purpose(raw: CredentialPurpose) -> CredentialPurpose {
    match raw {
        CredentialPurpose::SessionToken => CredentialPurpose::SessionTokenHash,
        CredentialPurpose::PasswordResetToken => CredentialPurpose::PasswordResetTokenHash,
        CredentialPurpose::CsrfToken => CredentialPurpose::CsrfTokenHash,
        _ => unreachable!("raw token purpose checked before hashing"),
    }
}

fn raw_purpose_for_hash(value_type: ValueType) -> Option<CredentialPurpose> {
    match value_type {
        ValueType::Credential(CredentialPurpose::SessionTokenHash) => {
            Some(CredentialPurpose::SessionToken)
        }
        ValueType::Credential(CredentialPurpose::PasswordResetTokenHash) => {
            Some(CredentialPurpose::PasswordResetToken)
        }
        ValueType::Credential(CredentialPurpose::CsrfTokenHash) => {
            Some(CredentialPurpose::CsrfToken)
        }
        _ => None,
    }
}

fn token_hash_contract_error() -> CompileError {
    CompileError::security(
        "SEC-A07-003",
        "tokenHash requires an issued Secret<SessionToken>, Secret<PasswordResetToken>, or Secret<CsrfToken>",
        Some("hash newly issued bearer tokens before persistence; do not hash arbitrary strings or presented request values".into()),
    )
}

fn token_contract_error(detail: &str) -> CompileError {
    CompileError::security(
        "SEC-A07-001",
        detail.to_string(),
        Some("persist tokenHash(issuedToken) in a purpose-matched Secret<...TokenHash> field and compare it with a validated presented token".into()),
    )
}

fn scalar(
    expr: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
    function: &str,
) -> Result<crate::scalar_security::ScalarType, CompileError> {
    infer_static_expr_type(expr, known, program)?
        .scalar()
        .ok_or_else(|| {
            CompileError::Syntax(format!("{function}(...) requires scalar credential values"))
        })
}

fn require_crypto_key(
    expr: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
    expected: CredentialPurpose,
    function: &str,
) -> Result<(), CompileError> {
    let scalar = scalar(expr, known, program, function)?;
    if scalar.value_type != ValueType::Credential(expected)
        || scalar.sensitivity != DataSensitivity::Secret
    {
        return Err(CompileError::security(
            "SEC-A04-020",
            format!("{function} requires Secret<{}>", expected.source_name()),
            Some("load a purpose-typed active key from a trusted secret/model boundary; do not use generic CryptoKey or arbitrary strings".into()),
        ));
    }
    Ok(())
}

fn require_verification_key(
    expr: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    let scalar = scalar(expr, known, program, "verifyWebhookSignature")?;
    let accepted = matches!(
        scalar.value_type,
        ValueType::Credential(CredentialPurpose::VerificationKeyWebhook)
            | ValueType::Credential(CredentialPurpose::RetiringVerificationKeyWebhook)
    );
    if !accepted || scalar.sensitivity != DataSensitivity::Secret {
        return Err(CompileError::security(
            "SEC-A04-021",
            "verifyWebhookSignature requires Secret<VerificationKey<Webhook>> or Secret<RetiringVerificationKey<Webhook>>",
            Some("verification may use active or retiring keys during rotation; retired keys are never accepted".into()),
        ));
    }
    Ok(())
}

fn require_decryption_key(
    expr: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    let scalar = scalar(expr, known, program, "decryptUserData")?;
    let accepted = matches!(
        scalar.value_type,
        ValueType::Credential(CredentialPurpose::EncryptionKeyUserData)
            | ValueType::Credential(CredentialPurpose::RetiringEncryptionKeyUserData)
    );
    if !accepted || scalar.sensitivity != DataSensitivity::Secret {
        return Err(CompileError::security(
            "SEC-A04-024",
            "decryptUserData requires Secret<EncryptionKey<UserData>> or Secret<RetiringEncryptionKey<UserData>>",
            Some("decryption may use active or retiring keys during rotation; retired keys are never accepted".into()),
        ));
    }
    Ok(())
}

fn require_sensitive_string(
    expr: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
    label: &str,
) -> Result<(), CompileError> {
    let scalar = scalar(expr, known, program, label)?;
    if scalar.value_type != ValueType::String
        || !matches!(
            scalar.sensitivity,
            DataSensitivity::Sensitive | DataSensitivity::Secret
        )
    {
        return Err(CompileError::security(
            "SEC-A04-025",
            format!("{label} must be Sensitive<String> or Secret<String>"),
            Some("encrypt classified user data explicitly; generic public strings do not need the UserData encryption boundary".into()),
        ));
    }
    Ok(())
}
fn require_public_string(
    expr: &Expr,
    known: &HashMap<String, StaticType>,
    program: &Program,
    label: &str,
) -> Result<(), CompileError> {
    let scalar = scalar(expr, known, program, label)?;
    if scalar.value_type != ValueType::String || scalar.sensitivity != DataSensitivity::Public {
        return Err(CompileError::security(
            "SEC-A04-022",
            format!("{label} must be public String data"),
            Some("serialize the intended public payload explicitly before cryptographic signing or verification".into()),
        ));
    }
    Ok(())
}
