use crate::redis_store::validate_redis_config;
use crate::{DataError, RedisConfig};

#[derive(Clone)]
pub struct RedisStore;

impl RedisStore {
    pub async fn connect(config: RedisConfig) -> Result<Self, DataError> {
        validate_redis_config(&config)?;
        Err(DataError::RedisUnavailable)
    }
    pub async fn ping(&self) -> Result<(), DataError> {
        Err(DataError::RedisUnavailable)
    }
    pub async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, DataError> {
        Err(DataError::RedisUnavailable)
    }
    pub async fn set(
        &self,
        _key: &str,
        _value: &[u8],
        _ttl_secs: Option<u64>,
    ) -> Result<(), DataError> {
        Err(DataError::RedisUnavailable)
    }
    pub async fn delete(&self, _key: &str) -> Result<bool, DataError> {
        Err(DataError::RedisUnavailable)
    }
    pub async fn get_delete(&self, _key: &str) -> Result<Option<Vec<u8>>, DataError> {
        Err(DataError::RedisUnavailable)
    }
    pub async fn increment(&self, _key: &str, _delta: i64) -> Result<i64, DataError> {
        Err(DataError::RedisUnavailable)
    }
    pub async fn set_if_absent(
        &self,
        _key: &str,
        _value: &[u8],
        _ttl_secs: u64,
    ) -> Result<bool, DataError> {
        Err(DataError::RedisUnavailable)
    }
    pub async fn increment_windowed(
        &self,
        _key: &str,
        _window_secs: u64,
    ) -> Result<i64, DataError> {
        Err(DataError::RedisUnavailable)
    }
}
