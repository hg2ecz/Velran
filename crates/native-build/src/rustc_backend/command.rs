#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use super::{BuildError, OptimizationProfile, RustcConfig, diagnostics, io_error};

pub(super) fn run_rustc(
    config: &RustcConfig,
    args: Vec<OsString>,
    stage: &'static str,
) -> Result<(), BuildError> {
    if config.repro_diagnostics() {
        eprintln!(
            "VELRAN_RUSTC_COMMAND={}",
            diagnostics::reproducible_command(config, &args)
        );
    }
    let output = Command::new(config.rustc_path())
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("RUSTDOCFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_HOME")
        .args(&args)
        .output()
        .map_err(|error| io_error("start isolated rustc", error))?;
    if !output.status.success() {
        let diagnostics = bounded_rustc_output(&output.stdout, &output.stderr);
        eprintln!("Velran rustc build failed ({stage}):");
        eprintln!("{diagnostics}");
        return Err(BuildError::RustcFailed {
            stage,
            status: output.status,
            diagnostics,
        });
    }
    Ok(())
}

fn bounded_rustc_output(stdout: &[u8], stderr: &[u8]) -> String {
    const MAX_BYTES: usize = 64 * 1024;
    let mut bytes = Vec::with_capacity((stdout.len() + stderr.len()).min(MAX_BYTES));
    if !stderr.is_empty() {
        bytes.extend_from_slice(&stderr[..stderr.len().min(MAX_BYTES)]);
    }
    if !stdout.is_empty() && bytes.len() < MAX_BYTES {
        if !bytes.is_empty() && !bytes.ends_with(b"\n") {
            bytes.push(b'\n');
        }
        let room = MAX_BYTES - bytes.len();
        bytes.extend_from_slice(&stdout[..stdout.len().min(room)]);
    }
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    text.retain(|ch| ch == '\n' || ch == '\t' || !ch.is_control());
    if stdout.len().saturating_add(stderr.len()) > MAX_BYTES {
        text.push_str("\n... rustc diagnostics truncated by Velran ...\n");
    }
    text
}

pub(super) fn cdylib_args(
    crate_name: &str,
    source: &Path,
    output: &Path,
    config: &RustcConfig,
) -> Vec<OsString> {
    let mut args = common_args(crate_name, config);
    args.extend([
        "--crate-type".into(),
        "cdylib".into(),
        "-o".into(),
        output.as_os_str().to_owned(),
        source.as_os_str().to_owned(),
    ]);
    args
}

fn common_args(crate_name: &str, config: &RustcConfig) -> Vec<OsString> {
    let opt = match config.optimization() {
        OptimizationProfile::Interactive => "2",
        OptimizationProfile::Release => "3",
    };
    let mut args = vec![
        "--crate-name".into(),
        crate_name.into(),
        "--edition".into(),
        "2024".into(),
        "--color".into(),
        "never".into(),
        "-C".into(),
        format!("opt-level={opt}").into(),
        "-C".into(),
        "panic=abort".into(),
        "-C".into(),
        "overflow-checks=yes".into(),
        "-C".into(),
        "debuginfo=0".into(),
    ];
    if config.optimization() == OptimizationProfile::Release {
        args.extend([
            "-C".into(),
            "codegen-units=1".into(),
            "-C".into(),
            "lto=thin".into(),
        ]);
    }
    if !config.toolchain().target_cpu.is_empty() {
        args.extend([
            "-C".into(),
            format!("target-cpu={}", config.toolchain().target_cpu).into(),
        ]);
    }
    if !config.toolchain().cpu_features.is_empty() {
        args.extend([
            "-C".into(),
            format!("target-feature={}", config.toolchain().cpu_features).into(),
        ]);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rustc_backend::ToolchainIdentity;
    use std::path::PathBuf;

    fn config(profile: OptimizationProfile, cpu: &str, features: &str) -> RustcConfig {
        RustcConfig::new(
            PathBuf::from("/tmp/rustc"),
            profile,
            ToolchainIdentity {
                rustc_version: "rustc-test".into(),
                target_triple: "x86_64-test".into(),
                target_cpu: cpu.into(),
                cpu_features: features.into(),
            },
        )
        .unwrap()
    }

    #[test]
    fn interactive_profile_uses_opt_level_two() {
        let args = common_args(
            "velran_shard_test",
            &config(OptimizationProfile::Interactive, "", ""),
        );
        assert!(args.iter().any(|arg| arg == "opt-level=2"));
        assert!(args.iter().any(|arg| arg == "panic=abort"));
        assert!(!args.iter().any(|arg| arg == "codegen-units=1"));
        assert!(!args.iter().any(|arg| arg == "lto=thin"));
    }

    #[test]
    fn release_profile_prefers_runtime_speed() {
        let args = common_args(
            "velran_shard_test",
            &config(OptimizationProfile::Release, "", ""),
        );
        assert!(args.iter().any(|arg| arg == "opt-level=3"));
        assert!(args.iter().any(|arg| arg == "codegen-units=1"));
        assert!(args.iter().any(|arg| arg == "lto=thin"));
    }

    #[test]
    fn configured_cpu_features_are_passed_to_rustc() {
        let args = common_args(
            "velran_shard_test",
            &config(OptimizationProfile::Release, "", "+sse2,+avx2"),
        );
        assert!(args.iter().any(|arg| arg == "target-feature=+sse2,+avx2"));
    }

    #[test]
    fn configured_target_cpu_is_passed_to_rustc() {
        let args = common_args(
            "velran_shard_test",
            &config(OptimizationProfile::Release, "native", ""),
        );
        assert!(args.iter().any(|arg| arg == "target-cpu=native"));
    }

    #[test]
    fn direct_build_is_one_cdylib_rustc_invocation_shape() {
        let args = cdylib_args(
            "velran_shard_test",
            Path::new("generated.rs"),
            Path::new("generated.so"),
            &config(OptimizationProfile::Interactive, "", ""),
        );
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "--crate-type" && pair[1] == "cdylib")
        );
        assert!(!args.iter().any(|arg| arg == "--extern"));
        assert!(!args.iter().any(|arg| arg == "rlib"));
    }
}
