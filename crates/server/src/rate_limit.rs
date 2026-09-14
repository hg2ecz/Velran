use data::RedisStore;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RateScope {
    Ip,
    Route,
    IpRoute,
    User,
    UserRoute,
}

#[derive(Clone, Debug)]
pub(super) struct RatePolicy {
    pub(super) limit: u64,
    pub(super) window_secs: u64,
    pub(super) scope: RateScope,
}

#[derive(Debug)]
pub(super) enum RateLimitError {
    UnknownPolicy(String),
    PrincipalRequired,
    Clock(SystemTimeError),
    Backend(String),
    LockPoisoned,
    CapacityExceeded,
}

impl fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPolicy(name) => write!(f, "unknown rate policy `{name}`"),
            Self::PrincipalRequired => {
                f.write_str("user-scoped rate policy requires authenticated principal")
            }
            Self::Clock(err) => write!(f, "system clock error: {err}"),
            Self::Backend(err) => write!(f, "rate-limit backend error: {err}"),
            Self::LockPoisoned => f.write_str("rate limiter lock poisoned"),
            Self::CapacityExceeded => f.write_str("memory rate limiter capacity exceeded"),
        }
    }
}

impl Error for RateLimitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Clock(err) => Some(err),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(super) struct RouteRateLimiter {
    pub(super) policies: Arc<HashMap<String, RatePolicy>>,
    pub(super) redis: Option<RedisStore>,
    pub(super) memory: Arc<Mutex<HashMap<String, (u64, u64)>>>,
}

impl RouteRateLimiter {
    pub(super) async fn check(
        &self,
        policy_name: &str,
        route_name: &str,
        peer: &str,
        principal: Option<&str>,
    ) -> Result<(bool, u64), RateLimitError> {
        let policy = self
            .policies
            .get(policy_name)
            .ok_or_else(|| RateLimitError::UnknownPolicy(policy_name.to_owned()))?;
        let subject = match policy.scope {
            RateScope::Ip => format!("ip:{peer}"),
            RateScope::Route => format!("route:{route_name}"),
            RateScope::IpRoute => format!("route:{route_name}:ip:{peer}"),
            RateScope::User => format!(
                "user:{}",
                principal.ok_or(RateLimitError::PrincipalRequired)?
            ),
            RateScope::UserRoute => format!(
                "route:{route_name}:user:{}",
                principal.ok_or(RateLimitError::PrincipalRequired)?
            ),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(RateLimitError::Clock)?
            .as_secs();
        let bucket = now / policy.window_secs;
        let expires_at = (bucket + 1).saturating_mul(policy.window_secs);
        let raw = format!(
            "rate:{policy_name}:{bucket}:{}",
            super::stable_key_hash(&subject)
        );
        let count = if let Some(redis) = &self.redis {
            redis
                .increment_windowed(&raw, policy.window_secs.saturating_add(1))
                .await
                .map_err(|err| RateLimitError::Backend(err.to_string()))? as u64
        } else {
            let mut map = self
                .memory
                .lock()
                .map_err(|_| RateLimitError::LockPoisoned)?;
            if map.len() >= 100_000 {
                map.retain(|_, (expires, _)| *expires > now);
            }
            if map.len() >= 100_000 && !map.contains_key(&raw) {
                return Err(RateLimitError::CapacityExceeded);
            }
            let entry = map.entry(raw).or_insert((expires_at, 0));
            entry.1 = entry.1.saturating_add(1);
            entry.1
        };
        Ok((count <= policy.limit, policy.window_secs))
    }
}
