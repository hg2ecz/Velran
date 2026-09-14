use crate::server_config_file::{
    FileNative, NativeCliConfig, NativeOptimizationCli, config_abs_path,
};
use crate::server_errors::CliParseError;

pub(super) fn apply_file(
    config: &mut NativeCliConfig,
    file: FileNative,
) -> Result<(), CliParseError> {
    if let Some(value) = file.enabled {
        config.enabled = value;
    }
    if let Some(value) = file.required {
        config.required = value;
    }
    if let Some(value) = file.debug_rustc_repro {
        config.debug_rustc_repro = value;
    }
    if let Some(value) = file.rustc {
        config.rustc = Some(config_abs_path(&value, "native.rustc")?);
    }
    if let Some(value) = file.cache_dir {
        config.cache_dir = Some(config_abs_path(&value, "native.cache_dir")?);
    }
    if let Some(value) = file.optimization {
        config.optimization = match value.as_str() {
            "interactive" => NativeOptimizationCli::Interactive,
            "release" => NativeOptimizationCli::Release,
            _ => {
                return Err(
                    "config `native.optimization` must be `interactive` or `release`".into(),
                );
            }
        };
    }
    if let Some(value) = file.target_cpu {
        if value.len() > 128
            || value
                .chars()
                .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        {
            return Err("config `native.target_cpu` is invalid".into());
        }
        config.target_cpu = Some(value);
    }
    if let Some(value) = file.cpu_features {
        if value.len() > 512
            || value
                .chars()
                .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, '+' | ',' | '-' | '_' | '.')))
        {
            return Err("config `native.cpu_features` is invalid".into());
        }
        config.cpu_features = Some(value);
    }
    Ok(())
}
