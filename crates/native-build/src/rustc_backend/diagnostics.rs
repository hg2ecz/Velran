use super::RustcConfig;
use std::ffi::{OsStr, OsString};
use std::path::Path;

pub(crate) fn print_generated_source(path: &Path, source: &str) {
    eprintln!("VELRAN_RUSTC_SOURCE={}", path.display());
    eprintln!("----- BEGIN VELRAN GENERATED RUST {} -----", path.display());
    eprint!("{}", source);
    if !source.ends_with('\n') {
        eprintln!();
    }
    eprintln!("----- END VELRAN GENERATED RUST -----");
}

pub(crate) fn reproducible_command(config: &RustcConfig, args: &[OsString]) -> String {
    let mut parts = vec![
        "env".to_owned(),
        "-u RUSTFLAGS".to_owned(),
        "-u RUSTC_WRAPPER".to_owned(),
        "-u RUSTC_WORKSPACE_WRAPPER".to_owned(),
        "-u RUSTDOCFLAGS".to_owned(),
        "-u CARGO_ENCODED_RUSTFLAGS".to_owned(),
        "-u CARGO_HOME".to_owned(),
        shell_quote(config.rustc_path().as_os_str()),
    ];
    parts.extend(args.iter().map(|arg| shell_quote(arg.as_os_str())));
    parts.join(" ")
}

fn shell_quote(value: &OsStr) -> String {
    let text = value.to_string_lossy();
    if !text.is_empty()
        && text.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'/' | b'_' | b'-' | b'.' | b'=' | b'+' | b',')
        })
    {
        return text.into_owned();
    }
    format!("'{}'", text.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rustc_backend::{OptimizationProfile, ToolchainIdentity};
    use std::path::PathBuf;

    #[test]
    fn command_contains_isolated_environment() {
        let config = RustcConfig::new(
            PathBuf::from("/tmp/rustc"),
            OptimizationProfile::Release,
            ToolchainIdentity {
                rustc_version: "rustc-test".into(),
                target_triple: "x86_64-test".into(),
                target_cpu: String::new(),
                cpu_features: String::new(),
            },
        )
        .unwrap()
        .with_repro_diagnostics(true);
        let args = vec![
            "--crate-type".into(),
            "cdylib".into(),
            "generated.rs".into(),
        ];
        let command = reproducible_command(&config, &args);
        assert!(command.contains("env -u RUSTFLAGS"));
        assert!(command.contains("/tmp/rustc"));
        assert!(command.contains("--crate-type cdylib generated.rs"));
    }
}
