use crate::AuthError;
use data::RedisStore;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthAbuseStage {
    Password,
    Mfa,
    PasswordReset,
}

impl AuthAbuseStage {
    fn key(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Mfa => "mfa",
            Self::PasswordReset => "reset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthAbusePolicy {
    pub pair_max_attempts: u32,
    pub principal_max_attempts: u32,
    pub source_max_attempts: u32,
    pub mfa_max_attempts: u32,
    pub window_secs: u64,
}

impl AuthAbusePolicy {
    pub fn validate(self) -> Result<Self, AuthError> {
        if self.pair_max_attempts == 0
            || self.principal_max_attempts == 0
            || self.source_max_attempts == 0
            || self.mfa_max_attempts == 0
            || self.window_secs == 0
        {
            return Err(AuthError::Internal);
        }
        Ok(self)
    }

    fn pair_limit(self, stage: AuthAbuseStage) -> u32 {
        match stage {
            AuthAbuseStage::Mfa => self.mfa_max_attempts,
            _ => self.pair_max_attempts,
        }
    }

    fn principal_limit(self, stage: AuthAbuseStage) -> u32 {
        match stage {
            AuthAbuseStage::Mfa => self.mfa_max_attempts,
            _ => self.principal_max_attempts,
        }
    }
}

#[derive(Clone)]
pub struct AuthAbuseLimiter {
    redis: Option<RedisStore>,
    memory: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
    policy: AuthAbusePolicy,
}

impl AuthAbuseLimiter {
    pub fn memory(policy: AuthAbusePolicy) -> Result<Self, AuthError> {
        Ok(Self {
            redis: None,
            memory: Arc::new(Mutex::new(HashMap::new())),
            policy: policy.validate()?,
        })
    }

    pub fn redis(store: RedisStore, policy: AuthAbusePolicy) -> Result<Self, AuthError> {
        Ok(Self {
            redis: Some(store),
            memory: Arc::new(Mutex::new(HashMap::new())),
            policy: policy.validate()?,
        })
    }

    pub async fn check(
        &self,
        stage: AuthAbuseStage,
        source: &str,
        principal: &str,
    ) -> Result<(), AuthError> {
        let dimensions = self.dimensions(stage, source, principal);
        for (key, max) in dimensions {
            if self.current(&key).await? >= max {
                return Err(AuthError::RateLimited);
            }
        }
        Ok(())
    }

    pub async fn record_failure(
        &self,
        stage: AuthAbuseStage,
        source: &str,
        principal: &str,
    ) -> Result<(), AuthError> {
        let dimensions = self.dimensions(stage, source, principal);
        let mut limited = false;
        for (key, max) in dimensions {
            if self.increment(&key).await? > max {
                limited = true;
            }
        }
        if limited {
            Err(AuthError::RateLimited)
        } else {
            Ok(())
        }
    }

    pub async fn clear_pair(
        &self,
        stage: AuthAbuseStage,
        source: &str,
        principal: &str,
    ) -> Result<(), AuthError> {
        let key = self.pair_key(stage, source, principal);
        if let Some(redis) = &self.redis {
            redis
                .delete(&key)
                .await
                .map_err(|_| AuthError::StoreUnavailable)?;
            return Ok(());
        }
        self.memory
            .lock()
            .map_err(|_| AuthError::Internal)?
            .remove(&key);
        Ok(())
    }

    fn dimensions(
        &self,
        stage: AuthAbuseStage,
        source: &str,
        principal: &str,
    ) -> [(String, u32); 3] {
        [
            (
                self.pair_key(stage, source, principal),
                self.policy.pair_limit(stage),
            ),
            (
                self.principal_key(stage, principal),
                self.policy.principal_limit(stage),
            ),
            (
                self.source_key(stage, source),
                self.policy.source_max_attempts,
            ),
        ]
    }

    fn pair_key(&self, stage: AuthAbuseStage, source: &str, principal: &str) -> String {
        format!(
            "auth-abuse:{}:pair:{}:{}",
            stage.key(),
            safe_key_component(source),
            safe_key_component(principal)
        )
    }

    fn principal_key(&self, stage: AuthAbuseStage, principal: &str) -> String {
        format!(
            "auth-abuse:{}:principal:{}",
            stage.key(),
            safe_key_component(principal)
        )
    }

    fn source_key(&self, stage: AuthAbuseStage, source: &str) -> String {
        format!(
            "auth-abuse:{}:source:{}",
            stage.key(),
            safe_key_component(source)
        )
    }

    async fn current(&self, key: &str) -> Result<u32, AuthError> {
        if let Some(redis) = &self.redis {
            let raw = redis
                .get(key)
                .await
                .map_err(|_| AuthError::StoreUnavailable)?;
            let Some(raw) = raw else { return Ok(0) };
            let text = std::str::from_utf8(&raw).map_err(|_| AuthError::StoreUnavailable)?;
            return text.parse::<u32>().map_err(|_| AuthError::StoreUnavailable);
        }
        let mut memory = self.memory.lock().map_err(|_| AuthError::Internal)?;
        let now = Instant::now();
        match memory.get(key).copied() {
            Some((started, count))
                if now.duration_since(started) < Duration::from_secs(self.policy.window_secs) =>
            {
                Ok(count)
            }
            Some(_) => {
                memory.remove(key);
                Ok(0)
            }
            None => Ok(0),
        }
    }

    async fn increment(&self, key: &str) -> Result<u32, AuthError> {
        if let Some(redis) = &self.redis {
            let count = redis
                .increment_windowed(key, self.policy.window_secs)
                .await
                .map_err(|_| AuthError::StoreUnavailable)?;
            return u32::try_from(count).map_err(|_| AuthError::StoreUnavailable);
        }
        let mut memory = self.memory.lock().map_err(|_| AuthError::Internal)?;
        let now = Instant::now();
        let entry = memory.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) >= Duration::from_secs(self.policy.window_secs) {
            *entry = (now, 0);
        }
        entry.1 = entry.1.saturating_add(1);
        Ok(entry.1)
    }
}

fn safe_key_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> AuthAbusePolicy {
        AuthAbusePolicy {
            pair_max_attempts: 2,
            principal_max_attempts: 3,
            source_max_attempts: 4,
            mfa_max_attempts: 2,
            window_secs: 60,
        }
    }

    #[tokio::test]
    async fn distributed_guessing_hits_principal_limit() {
        let limiter = AuthAbuseLimiter::memory(policy()).unwrap();
        assert!(
            limiter
                .record_failure(AuthAbuseStage::Password, "a", "alice")
                .await
                .is_ok()
        );
        assert!(
            limiter
                .record_failure(AuthAbuseStage::Password, "b", "alice")
                .await
                .is_ok()
        );
        assert!(
            limiter
                .record_failure(AuthAbuseStage::Password, "c", "alice")
                .await
                .is_ok()
        );
        assert!(matches!(
            limiter.check(AuthAbuseStage::Password, "d", "alice").await,
            Err(AuthError::RateLimited)
        ));
    }

    #[tokio::test]
    async fn credential_stuffing_hits_source_limit() {
        let limiter = AuthAbuseLimiter::memory(policy()).unwrap();
        for principal in ["a", "b", "c"] {
            assert!(
                limiter
                    .record_failure(AuthAbuseStage::Password, "source", principal)
                    .await
                    .is_ok()
            );
        }
        assert!(
            limiter
                .record_failure(AuthAbuseStage::Password, "source", "d")
                .await
                .is_ok()
        );
        assert!(matches!(
            limiter.check(AuthAbuseStage::Password, "source", "e").await,
            Err(AuthError::RateLimited)
        ));
    }

    #[tokio::test]
    async fn successful_pair_clear_does_not_clear_source_or_principal_history() {
        let limiter = AuthAbuseLimiter::memory(policy()).unwrap();
        limiter
            .record_failure(AuthAbuseStage::Password, "source", "alice")
            .await
            .unwrap();
        limiter
            .clear_pair(AuthAbuseStage::Password, "source", "alice")
            .await
            .unwrap();
        assert!(
            limiter
                .record_failure(AuthAbuseStage::Password, "other", "alice")
                .await
                .is_ok()
        );
        assert!(
            limiter
                .record_failure(AuthAbuseStage::Password, "third", "alice")
                .await
                .is_ok()
        );
        assert!(matches!(
            limiter
                .check(AuthAbuseStage::Password, "fourth", "alice")
                .await,
            Err(AuthError::RateLimited)
        ));
    }
}
