#![forbid(unsafe_code)]

mod bootstrap;
mod engine;
mod error;
mod report;
mod toolchain_identity_cache;
mod worker_policy;

pub use bootstrap::{
    BootstrapError, NativeOptimization, NativeRuntimeConfig, from_config, from_environment,
};
pub use engine::ApplicationRuntime;
pub use error::RuntimeError;
pub use report::RefreshReport;
pub use worker_policy::{CodeTrust, WorkerRequirement, WorkerSandboxProfile, worker_requirement};

#[cfg(test)]
mod tests {
    use super::{NativeOptimization, NativeRuntimeConfig};

    #[test]
    fn native_runtime_config_defaults_preserve_auto_discovery() {
        let config = NativeRuntimeConfig::default();
        assert!(config.enabled);
        assert!(config.rustc_path.is_none());
        assert!(config.cache_root.is_none());
        assert_eq!(config.optimization, NativeOptimization::Release);
        assert!(!config.debug_rustc_repro);
    }
}

#[cfg(test)]
mod cache_reuse_tests;
