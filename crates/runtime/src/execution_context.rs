use crate::errors::ResourceProfileError;
use language_core::AppError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    pub max_instructions: u64,
    pub max_allocated_bytes: u64,
    pub max_external_io_bytes: u64,
}
impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_instructions: 100_000,
            max_allocated_bytes: 32 * 1024 * 1024,
            max_external_io_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ResourceProfileConfig {
    pub max_instructions: u64,
    pub max_allocated_bytes: u64,
    pub max_external_io_bytes: u64,
    pub max_concurrent: usize,
}

struct ProfileRuntime {
    config: ResourceProfileConfig,
    semaphore: Arc<Semaphore>,
}
#[derive(Clone)]
pub struct ResourceProfiles {
    default: ResourceProfileConfig,
    default_semaphore: Arc<Semaphore>,
    profiles: Arc<HashMap<String, ProfileRuntime>>,
}
impl ResourceProfiles {
    pub fn new(
        default: ResourceProfileConfig,
        named: HashMap<String, ResourceProfileConfig>,
    ) -> Result<Self, ResourceProfileError> {
        if default.max_instructions == 0
            || default.max_allocated_bytes == 0
            || default.max_external_io_bytes == 0
            || default.max_concurrent == 0
        {
            return Err(ResourceProfileError::ZeroDefaultLimit);
        }
        let mut profiles = HashMap::new();
        for (name, config) in named {
            if name == "default"
                || name.is_empty()
                || !name.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric())
            {
                return Err(ResourceProfileError::InvalidName(name));
            }
            if config.max_instructions == 0
                || config.max_allocated_bytes == 0
                || config.max_external_io_bytes == 0
                || config.max_concurrent == 0
            {
                return Err(ResourceProfileError::ZeroNamedLimit(name));
            }
            profiles.insert(
                name,
                ProfileRuntime {
                    config,
                    semaphore: Arc::new(Semaphore::new(config.max_concurrent)),
                },
            );
        }
        Ok(Self {
            default,
            default_semaphore: Arc::new(Semaphore::new(default.max_concurrent)),
            profiles: Arc::new(profiles),
        })
    }
    pub fn default_for_limits(limits: &ExecutionLimits) -> Self {
        Self::new(
            ResourceProfileConfig {
                max_instructions: limits.max_instructions,
                max_allocated_bytes: limits.max_allocated_bytes,
                max_external_io_bytes: limits.max_external_io_bytes,
                max_concurrent: usize::MAX / 2,
            },
            HashMap::new(),
        )
        .expect("valid default limits")
    }
    pub fn default_config(&self) -> ResourceProfileConfig {
        self.default
    }
    pub fn config(&self, name: &str) -> Option<ResourceProfileConfig> {
        self.profiles.get(name).map(|p| p.config)
    }
    pub async fn acquire(
        &self,
        name: Option<&str>,
    ) -> Result<(ResourceProfileConfig, OwnedSemaphorePermit), AppError> {
        let (config, semaphore) = match name {
            Some(name) => {
                let profile = self.profiles.get(name).ok_or(AppError::Internal)?;
                (profile.config, Arc::clone(&profile.semaphore))
            }
            None => (self.default, Arc::clone(&self.default_semaphore)),
        };
        let permit = semaphore
            .acquire_owned()
            .await
            .map_err(|_| AppError::Internal)?;
        Ok((config, permit))
    }
}
