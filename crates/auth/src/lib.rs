use data::RedisStore;
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;
use std::time::Duration;
use totp_rfc::{Secret, Totp, ValidationWindow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    InvalidCredentials,
    InvalidSecondFactor,
    ReplayDetected,
    RateLimited,
    SessionCapacity,
    SessionInvalid,
    LdapUnavailable,
    LdapPolicy,
    StoreUnavailable,
    Internal,
}
impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidCredentials => "invalid credentials",
            Self::InvalidSecondFactor => "invalid second factor",
            Self::ReplayDetected => "second factor replay detected",
            Self::RateLimited => "authentication rate limit exceeded",
            Self::SessionCapacity => "session capacity exceeded",
            Self::SessionInvalid => "invalid session",
            Self::LdapUnavailable => "directory service unavailable",
            Self::LdapPolicy => "LDAP security policy violation",
            Self::StoreUnavailable => "authentication store unavailable",
            Self::Internal => "internal authentication error",
        })
    }
}
impl std::error::Error for AuthError {}

pub fn random_hex(bytes: usize) -> String {
    let mut raw = vec![0u8; bytes];
    rand::fill(&mut raw[..]);
    let mut out = String::with_capacity(bytes * 2);
    for byte in raw {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

mod session;
mod tenant;
pub use session::{RedisSessionStore, SessionBackend, SessionFlash, SessionSnapshot, SessionStore};
pub use tenant::{TenantId, validate_memberships};

pub struct TotpReplayGuard {
    accepted: Mutex<HashMap<String, u64>>,
}
impl Default for TotpReplayGuard {
    fn default() -> Self {
        Self {
            accepted: Mutex::new(HashMap::new()),
        }
    }
}
impl TotpReplayGuard {
    pub fn verify(
        &self,
        credential: &str,
        secret: &[u8],
        unix: u64,
        code: &str,
    ) -> Result<(), AuthError> {
        let counter = verify_totp(secret, unix, code)?;
        let mut a = self.accepted.lock().map_err(|_| AuthError::Internal)?;
        if a.get(credential).is_some_and(|p| counter <= *p) {
            return Err(AuthError::ReplayDetected);
        }
        a.insert(credential.into(), counter);
        Ok(())
    }
}
pub fn verify_totp(secret_bytes: &[u8], unix_seconds: u64, code: &str) -> Result<u64, AuthError> {
    let secret = Secret::new(secret_bytes).map_err(|_| AuthError::Internal)?;
    let matched = Totp::default()
        .verify_window(
            &secret,
            unix_seconds,
            ValidationWindow::RFC_RECOMMENDED,
            code,
        )
        .map_err(|_| AuthError::InvalidSecondFactor)?
        .ok_or(AuthError::InvalidSecondFactor)?;
    Ok(matched.counter())
}
fn safe_key_component(v: &str) -> String {
    let mut out = String::with_capacity(v.len() * 2);
    for b in v.as_bytes() {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
    }
    out
}

pub async fn verify_totp_redis(
    redis: &RedisStore,
    credential: &str,
    secret: &[u8],
    unix: u64,
    code: &str,
) -> Result<(), AuthError> {
    let counter = verify_totp(secret, unix, code)?;
    let key = format!("totp-replay:{}:{}", safe_key_component(credential), counter);
    let ok = redis
        .set_if_absent(&key, b"1", 180)
        .await
        .map_err(|_| AuthError::StoreUnavailable)?;
    if ok {
        Ok(())
    } else {
        Err(AuthError::ReplayDetected)
    }
}

mod abuse;
pub use abuse::{AuthAbuseLimiter, AuthAbusePolicy, AuthAbuseStage};

#[derive(Clone)]
pub struct LdapConfig {
    pub url: String,
    pub search_base: String,
    pub username_attribute: String,
    pub service_bind_dn: String,
    pub service_bind_password: String,
    pub timeout: Duration,
}
impl LdapConfig {
    pub fn validate(&self) -> Result<(), AuthError> {
        if !self.url.to_ascii_lowercase().starts_with("ldaps://") {
            return Err(AuthError::LdapPolicy);
        }
        if self.username_attribute.is_empty()
            || !self
                .username_attribute
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(AuthError::LdapPolicy);
        }
        if self.search_base.is_empty() {
            return Err(AuthError::LdapPolicy);
        }
        Ok(())
    }
}
pub async fn authenticate_ldap(
    config: &LdapConfig,
    username: &str,
    password: &str,
) -> Result<String, AuthError> {
    if username.is_empty() || password.is_empty() {
        return Err(AuthError::InvalidCredentials);
    }
    config.validate()?;
    #[cfg(feature = "ldap")]
    {
        use ldap3::{LdapConnAsync, Scope, SearchEntry, ldap_escape};
        let operation = async {
            let (conn, mut ldap) = LdapConnAsync::new(&config.url)
                .await
                .map_err(|_| AuthError::LdapUnavailable)?;
            ldap3::drive!(conn);
            ldap.simple_bind(&config.service_bind_dn, &config.service_bind_password)
                .await
                .map_err(|_| AuthError::LdapUnavailable)?
                .success()
                .map_err(|_| AuthError::LdapUnavailable)?;
            let filter = format!("({}={})", config.username_attribute, ldap_escape(username));
            let (entries, _) = ldap
                .search(&config.search_base, Scope::Subtree, &filter, vec!["dn"])
                .await
                .map_err(|_| AuthError::LdapUnavailable)?
                .success()
                .map_err(|_| AuthError::LdapUnavailable)?;
            if entries.len() != 1 {
                let _ = ldap.unbind().await;
                return Err(AuthError::InvalidCredentials);
            }
            let dn = SearchEntry::construct(
                entries
                    .into_iter()
                    .next()
                    .ok_or(AuthError::InvalidCredentials)?,
            )
            .dn;
            ldap.simple_bind(&dn, password)
                .await
                .map_err(|_| AuthError::LdapUnavailable)?
                .success()
                .map_err(|_| AuthError::InvalidCredentials)?;
            let _ = ldap.unbind().await;
            Ok(dn)
        };
        return tokio::time::timeout(config.timeout, operation)
            .await
            .map_err(|_| AuthError::LdapUnavailable)?;
    }
    #[cfg(not(feature = "ldap"))]
    {
        let _ = (username, password);
        Err(AuthError::LdapUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csrf_bound() {
        let s = SessionStore::new(Duration::from_secs(60), 10);
        let a = s.create().unwrap();
        let b = s.create().unwrap();
        assert!(s.verify_csrf(&a.id, &a.csrf_token).unwrap());
        assert!(!s.verify_csrf(&a.id, &b.csrf_token).unwrap());
    }
    #[test]
    fn rotation_roles() {
        let s = SessionStore::new(Duration::from_secs(60), 10);
        let a = s.create().unwrap();
        let b = s
            .rotate_authenticated(&a.id, "alice".into(), true, vec!["Admin".into()], vec![], 1)
            .unwrap();
        assert!(s.get(&a.id).unwrap().is_none());
        assert!(b.has_role("Admin"));
    }
    #[test]
    fn ldap_tls() {
        let c = LdapConfig {
            url: "ldap://x".into(),
            search_base: "dc=x".into(),
            username_attribute: "uid".into(),
            service_bind_dn: "cn=s".into(),
            service_bind_password: "x".into(),
            timeout: Duration::from_secs(1),
        };
        assert_eq!(c.validate(), Err(AuthError::LdapPolicy));
    }
}

mod local_user;
pub use local_user::{LocalUserAuth, LocalUserStore};

#[cfg(test)]
mod m44_flash_session_tests {
    use super::*;

    #[test]
    fn memory_flash_is_consumed_exactly_once() {
        let store = SessionStore::new(Duration::from_secs(60), 8);
        let session = store.create().unwrap();
        store.set_flash(&session.id, "success", "Saved").unwrap();
        let first = store.take_flash(&session.id).unwrap().unwrap();
        assert_eq!(first.kind, "success");
        assert_eq!(first.message, "Saved");
        assert!(store.take_flash(&session.id).unwrap().is_none());
    }

    #[test]
    fn memory_flash_rejects_unsafe_shape() {
        let store = SessionStore::new(Duration::from_secs(60), 8);
        let session = store.create().unwrap();
        assert!(store.set_flash(&session.id, "other", "Saved").is_err());
        assert!(
            store
                .set_flash(&session.id, "success", "bad\nline")
                .is_err()
        );
    }
}

#[cfg(test)]
mod session_tests;
