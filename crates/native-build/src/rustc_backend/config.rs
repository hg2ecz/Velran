#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use crate::build_cache::CacheIdentity;

use super::BuildError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainIdentity {
    pub rustc_version: String,
    pub target_triple: String,
    pub target_cpu: String,
    pub cpu_features: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustcConfig {
    rustc_path: PathBuf,
    optimization: OptimizationProfile,
    toolchain: ToolchainIdentity,
    repro_diagnostics: bool,
}

impl RustcConfig {
    pub fn new(
        rustc_path: PathBuf,
        optimization: OptimizationProfile,
        toolchain: ToolchainIdentity,
    ) -> Result<Self, BuildError> {
        if !rustc_path.is_absolute() {
            return Err(BuildError::RustcPathMustBeAbsolute);
        }
        if toolchain.rustc_version.trim().is_empty() || toolchain.target_triple.trim().is_empty() {
            return Err(BuildError::IncompleteToolchainIdentity);
        }
        Ok(Self {
            rustc_path,
            optimization,
            toolchain,
            repro_diagnostics: false,
        })
    }

    pub fn rustc_path(&self) -> &Path {
        &self.rustc_path
    }
    pub fn optimization(&self) -> OptimizationProfile {
        self.optimization
    }
    pub fn toolchain(&self) -> &ToolchainIdentity {
        &self.toolchain
    }
    pub fn with_repro_diagnostics(mut self, enabled: bool) -> Self {
        self.repro_diagnostics = enabled;
        self
    }
    pub fn repro_diagnostics(&self) -> bool {
        self.repro_diagnostics
    }
    pub fn cache_identity(&self) -> CacheIdentity {
        CacheIdentity {
            rustc_version: self.toolchain.rustc_version.clone(),
            target_triple: self.toolchain.target_triple.clone(),
            target_cpu: self.toolchain.target_cpu.clone(),
            cpu_features: self.toolchain.cpu_features.clone(),
            optimization_profile: self.optimization.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationProfile {
    Interactive,
    Release,
}

impl OptimizationProfile {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Interactive => "interactive-o2",
            Self::Release => "release-o3-thinlto",
        }
    }
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) enum TestToolchainConfigError {
    Message(String),
    Build(BuildError),
}

#[cfg(test)]
impl std::fmt::Display for TestToolchainConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) => f.write_str(message),
            Self::Build(error) => error.fmt(f),
        }
    }
}
#[cfg(test)]
impl std::error::Error for TestToolchainConfigError {}

#[cfg(test)]
pub(crate) fn test_toolchain_config(
    optimization: OptimizationProfile,
) -> Result<RustcConfig, TestToolchainConfigError> {
    use std::process::Command;

    fn command_candidate() -> Option<PathBuf> {
        if let Some(value) = std::env::var_os("RUSTC") {
            let path = PathBuf::from(value);
            if path.is_absolute() || path.components().count() > 1 {
                return Some(path);
            }
        }
        std::env::var_os("PATH").and_then(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join(format!("rustc{}", std::env::consts::EXE_SUFFIX)))
                .find(|candidate| candidate.is_file())
        })
    }

    let launcher = command_candidate().ok_or_else(|| {
        TestToolchainConfigError::Message("rustc launcher not found in test environment".to_owned())
    })?;
    let sysroot_output = Command::new(&launcher)
        .args(["--print", "sysroot"])
        .output()
        .map_err(|error| {
            TestToolchainConfigError::Message(format!("invoke rustc --print sysroot: {error}"))
        })?;
    if !sysroot_output.status.success() {
        return Err(TestToolchainConfigError::Message(format!(
            "rustc --print sysroot failed with {}",
            sysroot_output.status
        )));
    }
    let sysroot = String::from_utf8(sysroot_output.stdout).map_err(|error| {
        TestToolchainConfigError::Message(format!("rustc sysroot is not UTF-8: {error}"))
    })?;
    let rustc_path = PathBuf::from(sysroot.trim())
        .join("bin")
        .join(format!("rustc{}", std::env::consts::EXE_SUFFIX));
    if !rustc_path.is_absolute() || !rustc_path.is_file() {
        return Err(TestToolchainConfigError::Message(format!(
            "resolved toolchain rustc is not a regular absolute file: {}",
            rustc_path.display()
        )));
    }

    let version_output = Command::new(&rustc_path)
        .arg("-vV")
        .output()
        .map_err(|error| {
            TestToolchainConfigError::Message(format!("invoke resolved rustc -vV: {error}"))
        })?;
    if !version_output.status.success() {
        return Err(TestToolchainConfigError::Message(format!(
            "resolved rustc -vV failed with {}",
            version_output.status
        )));
    }
    let text = String::from_utf8(version_output.stdout).map_err(|error| {
        TestToolchainConfigError::Message(format!("resolved rustc -vV is not UTF-8: {error}"))
    })?;
    let rustc_version = text.lines().next().unwrap_or_default().trim().to_owned();
    let target_triple = text
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap_or_default()
        .trim()
        .to_owned();
    if rustc_version.is_empty() || target_triple.is_empty() {
        return Err(TestToolchainConfigError::Message(
            "resolved rustc -vV omitted version or host target".to_owned(),
        ));
    }

    RustcConfig::new(
        rustc_path,
        optimization,
        ToolchainIdentity {
            rustc_version,
            target_triple,
            target_cpu: String::new(),
            cpu_features: String::new(),
        },
    )
    .map_err(TestToolchainConfigError::Build)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rustc_path_must_be_absolute() {
        let toolchain = ToolchainIdentity {
            rustc_version: "rustc-test".into(),
            target_triple: "x86_64-test".into(),
            target_cpu: String::new(),
            cpu_features: String::new(),
        };
        assert!(matches!(
            RustcConfig::new(
                PathBuf::from("rustc"),
                OptimizationProfile::Interactive,
                toolchain
            ),
            Err(BuildError::RustcPathMustBeAbsolute)
        ));
    }
}
