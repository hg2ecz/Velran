use crate::AuthError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantId(String);

impl TenantId {
    pub fn parse(raw: &str) -> Result<Self, AuthError> {
        let value = raw.trim();
        if value.is_empty()
            || value.len() > 128
            || !value.is_ascii()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(AuthError::SessionInvalid);
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn validate_memberships(values: &[TenantId]) -> Result<(), AuthError> {
    if values.len() > 64 {
        return Err(AuthError::SessionInvalid);
    }
    let mut seen = std::collections::HashSet::with_capacity(values.len());
    if values.iter().any(|value| !seen.insert(value.as_str())) {
        return Err(AuthError::SessionInvalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_ids_are_canonical_and_bounded() {
        assert_eq!(TenantId::parse("Acme_01").unwrap().as_str(), "acme_01");
        assert!(TenantId::parse("../acme").is_err());
        assert!(TenantId::parse("").is_err());
    }

    #[test]
    fn duplicate_memberships_are_rejected() {
        let tenant = TenantId::parse("acme").unwrap();
        assert!(validate_memberships(&[tenant.clone(), tenant]).is_err());
    }
}
