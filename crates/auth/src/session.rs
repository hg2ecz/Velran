use super::{AuthError, TenantId, random_hex, validate_memberships};
use data::RedisStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFlash {
    pub kind: String,
    pub message: String,
}
fn valid_flash(kind: &str, message: &str) -> bool {
    matches!(kind, "success" | "info" | "warning" | "error")
        && !message.is_empty()
        && message.len() <= 200
        && !message.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub id: String,
    pub csrf_token: String,
    pub principal: Option<String>,
    pub mfa_verified: bool,
    pub roles: Vec<String>,
    #[serde(default)]
    pub memberships: Vec<TenantId>,
    #[serde(default)]
    pub auth_generation: u64,
}
impl SessionSnapshot {
    pub fn is_authenticated(&self) -> bool {
        self.principal.is_some()
    }
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
    pub fn has_membership(&self, tenant: &TenantId) -> bool {
        self.memberships.iter().any(|value| value == tenant)
    }
}

#[derive(Clone)]
pub struct SessionStore {
    inner: Arc<Mutex<HashMap<String, SessionRecord>>>,
    ttl: Duration,
    max_sessions: usize,
}
#[derive(Clone)]
struct SessionRecord {
    csrf: String,
    expires_at: Instant,
    principal: Option<String>,
    mfa_verified: bool,
    roles: Vec<String>,
    memberships: Vec<TenantId>,
    auth_generation: u64,
    flash: Option<SessionFlash>,
}
impl SessionStore {
    pub fn new(ttl: Duration, max_sessions: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl,
            max_sessions,
        }
    }
    pub fn create(&self) -> Result<SessionSnapshot, AuthError> {
        let mut s = self.inner.lock().map_err(|_| AuthError::Internal)?;
        let now = Instant::now();
        s.retain(|_, r| r.expires_at > now);
        if s.len() >= self.max_sessions {
            return Err(AuthError::SessionCapacity);
        }
        let id = random_hex(32);
        let csrf = random_hex(32);
        s.insert(
            id.clone(),
            SessionRecord {
                csrf: csrf.clone(),
                expires_at: now + self.ttl,
                principal: None,
                mfa_verified: false,
                roles: vec![],
                memberships: vec![],
                auth_generation: 0,
                flash: None,
            },
        );
        Ok(SessionSnapshot {
            id,
            csrf_token: csrf,
            principal: None,
            mfa_verified: false,
            roles: vec![],
            memberships: vec![],
            auth_generation: 0,
        })
    }
    pub fn get(&self, id: &str) -> Result<Option<SessionSnapshot>, AuthError> {
        if !valid_session_id(id) {
            return Ok(None);
        }
        let mut s = self.inner.lock().map_err(|_| AuthError::Internal)?;
        let now = Instant::now();
        let Some(r) = s.get_mut(id) else {
            return Ok(None);
        };
        if r.expires_at <= now {
            s.remove(id);
            return Ok(None);
        }
        r.expires_at = now + self.ttl;
        Ok(Some(SessionSnapshot {
            id: id.into(),
            csrf_token: r.csrf.clone(),
            principal: r.principal.clone(),
            mfa_verified: r.mfa_verified,
            roles: r.roles.clone(),
            memberships: r.memberships.clone(),
            auth_generation: r.auth_generation,
        }))
    }
    pub fn verify_csrf(&self, id: &str, supplied: &str) -> Result<bool, AuthError> {
        let Some(s) = self.get(id)? else {
            return Ok(false);
        };
        Ok(ct_equal(&s.csrf_token, supplied))
    }
    pub fn set_flash(&self, id: &str, kind: &str, message: &str) -> Result<(), AuthError> {
        if !valid_session_id(id) || !valid_flash(kind, message) {
            return Err(AuthError::SessionInvalid);
        }
        let mut s = self.inner.lock().map_err(|_| AuthError::Internal)?;
        let now = Instant::now();
        let Some(record) = s.get_mut(id) else {
            return Err(AuthError::SessionInvalid);
        };
        if record.expires_at <= now {
            s.remove(id);
            return Err(AuthError::SessionInvalid);
        }
        record.flash = Some(SessionFlash {
            kind: kind.into(),
            message: message.into(),
        });
        Ok(())
    }
    pub fn take_flash(&self, id: &str) -> Result<Option<SessionFlash>, AuthError> {
        if !valid_session_id(id) {
            return Ok(None);
        }
        let mut s = self.inner.lock().map_err(|_| AuthError::Internal)?;
        let now = Instant::now();
        let Some(record) = s.get_mut(id) else {
            return Ok(None);
        };
        if record.expires_at <= now {
            s.remove(id);
            return Ok(None);
        }
        Ok(record.flash.take())
    }

    pub fn rotate_authenticated(
        &self,
        old: &str,
        principal: String,
        mfa: bool,
        roles: Vec<String>,
        memberships: Vec<TenantId>,
        auth_generation: u64,
    ) -> Result<SessionSnapshot, AuthError> {
        validate_memberships(&memberships)?;
        let mut s = self.inner.lock().map_err(|_| AuthError::Internal)?;
        s.remove(old);
        let now = Instant::now();
        s.retain(|_, r| r.expires_at > now);
        if s.len() >= self.max_sessions {
            return Err(AuthError::SessionCapacity);
        }
        let id = random_hex(32);
        let csrf = random_hex(32);
        s.insert(
            id.clone(),
            SessionRecord {
                csrf: csrf.clone(),
                expires_at: now + self.ttl,
                principal: Some(principal.clone()),
                mfa_verified: mfa,
                roles: roles.clone(),
                memberships: memberships.clone(),
                auth_generation,
                flash: None,
            },
        );
        Ok(SessionSnapshot {
            id,
            csrf_token: csrf,
            principal: Some(principal),
            mfa_verified: mfa,
            roles,
            memberships,
            auth_generation,
        })
    }
    pub fn invalidate(&self, id: &str) -> Result<(), AuthError> {
        self.inner
            .lock()
            .map_err(|_| AuthError::Internal)?
            .remove(id);
        Ok(())
    }
}

#[derive(Clone)]
pub struct RedisSessionStore {
    redis: RedisStore,
    ttl_secs: u64,
}
impl RedisSessionStore {
    pub fn new(redis: RedisStore, ttl_secs: u64) -> Self {
        Self { redis, ttl_secs }
    }
    pub async fn create(&self) -> Result<SessionSnapshot, AuthError> {
        for _ in 0..4 {
            let record = SessionSnapshot {
                id: random_hex(32),
                csrf_token: random_hex(32),
                principal: None,
                mfa_verified: false,
                roles: vec![],
                memberships: vec![],
                auth_generation: 0,
            };
            let bytes = serde_json::to_vec(&record).map_err(|_| AuthError::Internal)?;
            if self
                .redis
                .set_if_absent(&format!("session:{}", record.id), &bytes, self.ttl_secs)
                .await
                .map_err(|_| AuthError::StoreUnavailable)?
            {
                return Ok(record);
            }
        }
        Err(AuthError::Internal)
    }
    pub async fn get(&self, id: &str) -> Result<Option<SessionSnapshot>, AuthError> {
        if !valid_session_id(id) {
            return Ok(None);
        }
        let Some(bytes) = self
            .redis
            .get(&format!("session:{id}"))
            .await
            .map_err(|_| AuthError::StoreUnavailable)?
        else {
            return Ok(None);
        };
        let record: SessionSnapshot =
            serde_json::from_slice(&bytes).map_err(|_| AuthError::StoreUnavailable)?;
        if record.id != id {
            return Err(AuthError::StoreUnavailable);
        };
        self.redis
            .set(&format!("session:{id}"), &bytes, Some(self.ttl_secs))
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        Ok(Some(record))
    }
    pub async fn verify_csrf(&self, id: &str, supplied: &str) -> Result<bool, AuthError> {
        let Some(s) = self.get(id).await? else {
            return Ok(false);
        };
        Ok(ct_equal(&s.csrf_token, supplied))
    }
    pub async fn set_flash(&self, id: &str, kind: &str, message: &str) -> Result<(), AuthError> {
        if !valid_session_id(id) || !valid_flash(kind, message) {
            return Err(AuthError::SessionInvalid);
        }
        if self.get(id).await?.is_none() {
            return Err(AuthError::SessionInvalid);
        }
        let flash = SessionFlash {
            kind: kind.into(),
            message: message.into(),
        };
        let bytes = serde_json::to_vec(&flash).map_err(|_| AuthError::Internal)?;
        self.redis
            .set(&format!("session-flash:{id}"), &bytes, Some(self.ttl_secs))
            .await
            .map_err(|_| AuthError::StoreUnavailable)
    }
    pub async fn take_flash(&self, id: &str) -> Result<Option<SessionFlash>, AuthError> {
        if !valid_session_id(id) {
            return Ok(None);
        }
        let Some(bytes) = self
            .redis
            .get_delete(&format!("session-flash:{id}"))
            .await
            .map_err(|_| AuthError::StoreUnavailable)?
        else {
            return Ok(None);
        };
        let flash: SessionFlash =
            serde_json::from_slice(&bytes).map_err(|_| AuthError::StoreUnavailable)?;
        if !valid_flash(&flash.kind, &flash.message) {
            return Err(AuthError::StoreUnavailable);
        }
        Ok(Some(flash))
    }

    pub async fn rotate_authenticated(
        &self,
        old: &str,
        principal: String,
        mfa: bool,
        roles: Vec<String>,
        memberships: Vec<TenantId>,
        auth_generation: u64,
    ) -> Result<SessionSnapshot, AuthError> {
        validate_memberships(&memberships)?;
        let record = SessionSnapshot {
            id: random_hex(32),
            csrf_token: random_hex(32),
            principal: Some(principal),
            mfa_verified: mfa,
            roles,
            memberships,
            auth_generation,
        };
        let bytes = serde_json::to_vec(&record).map_err(|_| AuthError::Internal)?;
        if !self
            .redis
            .set_if_absent(&format!("session:{}", record.id), &bytes, self.ttl_secs)
            .await
            .map_err(|_| AuthError::StoreUnavailable)?
        {
            return Err(AuthError::Internal);
        };
        self.redis
            .delete(&format!("session:{old}"))
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        self.redis
            .delete(&format!("session-flash:{old}"))
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        Ok(record)
    }
    pub async fn invalidate(&self, id: &str) -> Result<(), AuthError> {
        self.redis
            .delete(&format!("session:{id}"))
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        self.redis
            .delete(&format!("session-flash:{id}"))
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        Ok(())
    }
}
fn valid_session_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn ct_equal(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[derive(Clone)]
pub enum SessionBackend {
    Memory(SessionStore),
    Redis(RedisSessionStore),
}
impl SessionBackend {
    pub async fn create(&self) -> Result<SessionSnapshot, AuthError> {
        match self {
            Self::Memory(s) => s.create(),
            Self::Redis(s) => s.create().await,
        }
    }
    pub async fn get(&self, id: &str) -> Result<Option<SessionSnapshot>, AuthError> {
        match self {
            Self::Memory(s) => s.get(id),
            Self::Redis(s) => s.get(id).await,
        }
    }
    pub async fn verify_csrf(&self, id: &str, v: &str) -> Result<bool, AuthError> {
        match self {
            Self::Memory(s) => s.verify_csrf(id, v),
            Self::Redis(s) => s.verify_csrf(id, v).await,
        }
    }
    pub async fn set_flash(&self, id: &str, kind: &str, message: &str) -> Result<(), AuthError> {
        match self {
            Self::Memory(s) => s.set_flash(id, kind, message),
            Self::Redis(s) => s.set_flash(id, kind, message).await,
        }
    }
    pub async fn take_flash(&self, id: &str) -> Result<Option<SessionFlash>, AuthError> {
        match self {
            Self::Memory(s) => s.take_flash(id),
            Self::Redis(s) => s.take_flash(id).await,
        }
    }

    pub async fn rotate_authenticated(
        &self,
        old: &str,
        p: String,
        m: bool,
        r: Vec<String>,
        memberships: Vec<TenantId>,
        auth_generation: u64,
    ) -> Result<SessionSnapshot, AuthError> {
        match self {
            Self::Memory(s) => s.rotate_authenticated(old, p, m, r, memberships, auth_generation),
            Self::Redis(s) => {
                s.rotate_authenticated(old, p, m, r, memberships, auth_generation)
                    .await
            }
        }
    }
    pub async fn invalidate(&self, id: &str) -> Result<(), AuthError> {
        match self {
            Self::Memory(s) => s.invalidate(id),
            Self::Redis(s) => s.invalidate(id).await,
        }
    }
}
