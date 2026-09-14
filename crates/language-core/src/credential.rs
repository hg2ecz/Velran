#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CredentialPurpose {
    Password,
    PasswordHash,
    ApiToken,
    SessionToken,
    SessionTokenHash,
    PasswordResetToken,
    PasswordResetTokenHash,
    CsrfToken,
    CsrfTokenHash,
    CryptoKey,
    SigningKeyWebhook,
    RetiringSigningKeyWebhook,
    RetiredSigningKeyWebhook,
    VerificationKeyWebhook,
    RetiringVerificationKeyWebhook,
    RetiredVerificationKeyWebhook,
    EncryptionKeyUserData,
    RetiringEncryptionKeyUserData,
    RetiredEncryptionKeyUserData,
}

impl CredentialPurpose {
    pub const fn source_name(self) -> &'static str {
        match self {
            Self::Password => "Password",
            Self::PasswordHash => "PasswordHash",
            Self::ApiToken => "ApiToken",
            Self::SessionToken => "SessionToken",
            Self::SessionTokenHash => "SessionTokenHash",
            Self::PasswordResetToken => "PasswordResetToken",
            Self::PasswordResetTokenHash => "PasswordResetTokenHash",
            Self::CsrfToken => "CsrfToken",
            Self::CsrfTokenHash => "CsrfTokenHash",
            Self::CryptoKey => "CryptoKey",
            Self::SigningKeyWebhook => "SigningKey<Webhook>",
            Self::RetiringSigningKeyWebhook => "RetiringSigningKey<Webhook>",
            Self::RetiredSigningKeyWebhook => "RetiredSigningKey<Webhook>",
            Self::VerificationKeyWebhook => "VerificationKey<Webhook>",
            Self::RetiringVerificationKeyWebhook => "RetiringVerificationKey<Webhook>",
            Self::RetiredVerificationKeyWebhook => "RetiredVerificationKey<Webhook>",
            Self::EncryptionKeyUserData => "EncryptionKey<UserData>",
            Self::RetiringEncryptionKeyUserData => "RetiringEncryptionKey<UserData>",
            Self::RetiredEncryptionKeyUserData => "RetiredEncryptionKey<UserData>",
        }
    }

    pub const fn is_crypto_key(self) -> bool {
        matches!(
            self,
            Self::CryptoKey
                | Self::SigningKeyWebhook
                | Self::RetiringSigningKeyWebhook
                | Self::RetiredSigningKeyWebhook
                | Self::VerificationKeyWebhook
                | Self::RetiringVerificationKeyWebhook
                | Self::RetiredVerificationKeyWebhook
                | Self::EncryptionKeyUserData
                | Self::RetiringEncryptionKeyUserData
                | Self::RetiredEncryptionKeyUserData
        )
    }

    pub const fn is_retired_crypto_key(self) -> bool {
        matches!(
            self,
            Self::RetiredSigningKeyWebhook
                | Self::RetiredVerificationKeyWebhook
                | Self::RetiredEncryptionKeyUserData
        )
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "Password" => Some(Self::Password),
            "PasswordHash" => Some(Self::PasswordHash),
            "ApiToken" => Some(Self::ApiToken),
            "SessionToken" => Some(Self::SessionToken),
            "SessionTokenHash" => Some(Self::SessionTokenHash),
            "PasswordResetToken" => Some(Self::PasswordResetToken),
            "PasswordResetTokenHash" => Some(Self::PasswordResetTokenHash),
            "CsrfToken" => Some(Self::CsrfToken),
            "CsrfTokenHash" => Some(Self::CsrfTokenHash),
            "CryptoKey" => Some(Self::CryptoKey),
            "SigningKey<Webhook>" => Some(Self::SigningKeyWebhook),
            "RetiringSigningKey<Webhook>" => Some(Self::RetiringSigningKeyWebhook),
            "RetiredSigningKey<Webhook>" => Some(Self::RetiredSigningKeyWebhook),
            "VerificationKey<Webhook>" => Some(Self::VerificationKeyWebhook),
            "RetiringVerificationKey<Webhook>" => Some(Self::RetiringVerificationKeyWebhook),
            "RetiredVerificationKey<Webhook>" => Some(Self::RetiredVerificationKeyWebhook),
            "EncryptionKey<UserData>" => Some(Self::EncryptionKeyUserData),
            "RetiringEncryptionKey<UserData>" => Some(Self::RetiringEncryptionKeyUserData),
            "RetiredEncryptionKey<UserData>" => Some(Self::RetiredEncryptionKeyUserData),
            _ => None,
        }
    }
}
