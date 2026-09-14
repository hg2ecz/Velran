use crate::DataError;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub namespace: String,
    pub connection_timeout: Duration,
    pub response_timeout: Duration,
    pub max_key_bytes: usize,
    pub max_value_bytes: usize,
    pub max_ttl_secs: u64,
    pub require_tls: bool,
}

impl RedisConfig {
    pub fn secure_default(url: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            namespace: namespace.into(),
            connection_timeout: Duration::from_secs(3),
            response_timeout: Duration::from_secs(3),
            max_key_bytes: 1024,
            max_value_bytes: 1024 * 1024,
            max_ttl_secs: 7 * 24 * 60 * 60,
            require_tls: true,
        }
    }
}

#[cfg(feature = "redis")]
mod enabled {
    use super::{DataError, RedisConfig, validate_redis_config};
    use redis::aio::{ConnectionManager, ConnectionManagerConfig};

    #[derive(Clone)]
    pub struct RedisStore {
        manager: ConnectionManager,
        namespace: String,
        max_key_bytes: usize,
        max_value_bytes: usize,
        max_ttl_secs: u64,
    }

    impl RedisStore {
        pub async fn connect(config: RedisConfig) -> Result<Self, DataError> {
            validate_redis_config(&config)?;
            let client = redis::Client::open(config.url.as_str()).map_err(DataError::Redis)?;
            let manager_config = ConnectionManagerConfig::new()
                .set_connection_timeout(Some(config.connection_timeout))
                .set_response_timeout(Some(config.response_timeout))
                .set_number_of_retries(3);
            let manager = ConnectionManager::new_with_config(client, manager_config)
                .await
                .map_err(DataError::Redis)?;
            Ok(Self {
                manager,
                namespace: config.namespace,
                max_key_bytes: config.max_key_bytes,
                max_value_bytes: config.max_value_bytes,
                max_ttl_secs: config.max_ttl_secs,
            })
        }

        pub async fn ping(&self) -> Result<(), DataError> {
            let mut con = self.manager.clone();
            let response: String = redis::cmd("PING")
                .query_async(&mut con)
                .await
                .map_err(DataError::Redis)?;
            if response != "PONG" {
                return Err(DataError::InvalidRedisResponse);
            }
            Ok(())
        }
        pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, DataError> {
            let key = self.key(key)?;
            let mut con = self.manager.clone();
            redis::cmd("GET")
                .arg(key)
                .query_async(&mut con)
                .await
                .map_err(DataError::Redis)
        }
        pub async fn set(
            &self,
            key: &str,
            value: &[u8],
            ttl_secs: Option<u64>,
        ) -> Result<(), DataError> {
            if value.len() > self.max_value_bytes {
                return Err(DataError::RedisValueTooLarge);
            }
            let key = self.key(key)?;
            let mut cmd = redis::cmd("SET");
            cmd.arg(key).arg(value);
            if let Some(ttl) = ttl_secs {
                if ttl == 0 || ttl > self.max_ttl_secs {
                    return Err(DataError::InvalidRedisTtl);
                }
                cmd.arg("EX").arg(ttl);
            }
            let mut con = self.manager.clone();
            let response: String = cmd.query_async(&mut con).await.map_err(DataError::Redis)?;
            if response != "OK" {
                return Err(DataError::InvalidRedisResponse);
            }
            Ok(())
        }
        pub async fn delete(&self, key: &str) -> Result<bool, DataError> {
            let key = self.key(key)?;
            let mut con = self.manager.clone();
            let removed: i64 = redis::cmd("DEL")
                .arg(key)
                .query_async(&mut con)
                .await
                .map_err(DataError::Redis)?;
            Ok(removed > 0)
        }
        pub async fn get_delete(&self, key: &str) -> Result<Option<Vec<u8>>, DataError> {
            let key = self.key(key)?;
            let mut con = self.manager.clone();
            let (value, _removed): (Option<Vec<u8>>, i64) = redis::pipe()
                .atomic()
                .cmd("GET")
                .arg(&key)
                .cmd("DEL")
                .arg(&key)
                .query_async(&mut con)
                .await
                .map_err(DataError::Redis)?;
            Ok(value)
        }
        pub async fn increment(&self, key: &str, delta: i64) -> Result<i64, DataError> {
            let key = self.key(key)?;
            let mut con = self.manager.clone();
            redis::cmd("INCRBY")
                .arg(key)
                .arg(delta)
                .query_async(&mut con)
                .await
                .map_err(DataError::Redis)
        }
        pub async fn set_if_absent(
            &self,
            key: &str,
            value: &[u8],
            ttl_secs: u64,
        ) -> Result<bool, DataError> {
            if value.len() > self.max_value_bytes {
                return Err(DataError::RedisValueTooLarge);
            }
            if ttl_secs == 0 || ttl_secs > self.max_ttl_secs {
                return Err(DataError::InvalidRedisTtl);
            }
            let key = self.key(key)?;
            let mut con = self.manager.clone();
            let response: Option<String> = redis::cmd("SET")
                .arg(key)
                .arg(value)
                .arg("EX")
                .arg(ttl_secs)
                .arg("NX")
                .query_async(&mut con)
                .await
                .map_err(DataError::Redis)?;
            Ok(response.as_deref() == Some("OK"))
        }
        pub async fn increment_windowed(
            &self,
            key: &str,
            window_secs: u64,
        ) -> Result<i64, DataError> {
            if window_secs == 0 || window_secs > self.max_ttl_secs {
                return Err(DataError::InvalidRedisTtl);
            }
            let key = self.key(key)?;
            let mut con = self.manager.clone();
            let (count, _expire): (i64, i64) = redis::pipe()
                .atomic()
                .cmd("INCR")
                .arg(&key)
                .cmd("EXPIRE")
                .arg(&key)
                .arg(window_secs)
                .query_async(&mut con)
                .await
                .map_err(DataError::Redis)?;
            Ok(count)
        }
        fn key(&self, raw: &str) -> Result<String, DataError> {
            if raw.is_empty() || raw.len() > self.max_key_bytes {
                return Err(DataError::InvalidRedisKey);
            }
            if raw.bytes().any(|b| b == 0 || b < 0x20 || b == 0x7f) {
                return Err(DataError::InvalidRedisKey);
            }
            Ok(format!("{}:{}", self.namespace, raw))
        }
    }
}

#[cfg(not(feature = "redis"))]
#[path = "redis_store_disabled.rs"]
mod disabled;

#[cfg(not(feature = "redis"))]
pub use disabled::RedisStore;
#[cfg(feature = "redis")]
pub use enabled::RedisStore;

pub(crate) fn validate_redis_config(config: &RedisConfig) -> Result<(), DataError> {
    if config.namespace.is_empty()
        || config.namespace.len() > 128
        || !config
            .namespace
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(DataError::InvalidRedisNamespace);
    }
    if config.require_tls && !config.url.starts_with("rediss://") {
        return Err(DataError::TlsRequired);
    }
    if !config.url.starts_with("redis://") && !config.url.starts_with("rediss://") {
        return Err(DataError::InvalidRedisUrl);
    }
    Ok(())
}
