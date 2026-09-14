use crate::server_config_file::{NativeCliConfig, NativeOptimizationCli};
use native_runtime::application_runtime::ApplicationRuntime;
use observability::server_event;
use std::path::Path;
use std::sync::Arc;

pub(super) type SharedNativeRuntime = Arc<ApplicationRuntime>;

pub(super) fn initialize(
    app: &Path,
    config: &NativeCliConfig,
) -> Result<Option<SharedNativeRuntime>, native_runtime::application_runtime::RuntimeError> {
    if !config.enabled {
        server_event(
            "info",
            "native_runtime_disabled",
            "server",
            "disabled by [native].enabled=false",
        );
        return Ok(None);
    }
    let settings = native_runtime::application_runtime::NativeRuntimeConfig {
        enabled: true,
        rustc_path: config.rustc.clone(),
        cache_root: config.cache_dir.clone(),
        optimization: match config.optimization {
            NativeOptimizationCli::Interactive => {
                native_runtime::application_runtime::NativeOptimization::Interactive
            }
            NativeOptimizationCli::Release => {
                native_runtime::application_runtime::NativeOptimization::Release
            }
        },
        target_cpu: config.target_cpu.clone(),
        cpu_features: config.cpu_features.clone(),
        required: config.required,
        debug_rustc_repro: config.debug_rustc_repro,
    };
    match native_runtime::application_runtime::from_config(app, &settings) {
        Ok(runtime) => {
            let generation = runtime.generation()?;
            let native_shards = runtime.native_shard_count()?;
            server_event(
                "info",
                "native_runtime_ready",
                "server",
                &format!(
                    "entry={} generation={} native_shards={} required={}",
                    app.display(),
                    generation,
                    native_shards,
                    config.required
                ),
            );
            Ok(Some(Arc::new(runtime)))
        }
        Err(error) => {
            server_event(
                "warn",
                "native_runtime_unavailable",
                "server",
                &error.to_string(),
            );
            if config.required {
                Err(error)
            } else {
                Ok(None)
            }
        }
    }
}
