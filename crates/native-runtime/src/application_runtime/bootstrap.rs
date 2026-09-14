use native_build::build_cache::NativeBuildCache;
use native_build::incremental_build::IncrementalBuilder;
use native_build::rustc_backend::{OptimizationProfile, RustcConfig, ToolchainIdentity};
use std::env;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::toolchain_identity_cache::{self, RustcProbeKey};
use super::{ApplicationRuntime, RuntimeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRuntimeConfig {
    pub enabled: bool,
    pub rustc_path: Option<PathBuf>,
    pub cache_root: Option<PathBuf>,
    pub optimization: NativeOptimization,
    pub target_cpu: Option<String>,
    pub cpu_features: Option<String>,
    pub required: bool,
    pub debug_rustc_repro: bool,
}

impl Default for NativeRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rustc_path: None,
            cache_root: None,
            optimization: NativeOptimization::Release,
            target_cpu: None,
            cpu_features: None,
            required: false,
            debug_rustc_repro: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOptimization {
    Interactive,
    Release,
}

impl NativeOptimization {
    fn rustc_profile(self) -> OptimizationProfile {
        match self {
            Self::Interactive => OptimizationProfile::Interactive,
            Self::Release => OptimizationProfile::Release,
        }
    }
}

#[derive(Debug)]
pub enum BootstrapError {
    Disabled,
    MissingAppParent,
    EntryIo(io::ErrorKind),
    RustcNotFound,
    RustcPathNotAbsolute,
    RustcIo(io::ErrorKind),
    RustcFailed,
    RustcOutputInvalid,
    RustcIdentityIncomplete,
    BuildConfig,
    Cache,
    Incremental,
    NoNativeShards,
}
impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Disabled => "native runtime disabled by configuration",
            Self::MissingAppParent => "application path has no parent",
            Self::EntryIo(_) => "application path canonicalization failed",
            Self::RustcNotFound => "rustc not found; native runtime disabled",
            Self::RustcPathNotAbsolute => "RUSTC must be an absolute path",
            Self::RustcIo(_) => "rustc invocation failed",
            Self::RustcFailed => "rustc -vV returned failure",
            Self::RustcOutputInvalid => "rustc -vV output is invalid",
            Self::RustcIdentityIncomplete => "rustc -vV missing version or host target",
            Self::BuildConfig => "native rustc configuration is invalid",
            Self::Cache => "native build cache initialization failed",
            Self::Incremental => "incremental native builder initialization failed",
            Self::NoNativeShards => {
                "native runtime required but no handler shard was compiled and loaded"
            }
        })
    }
}
impl std::error::Error for BootstrapError {}

pub fn from_environment(entry: &Path) -> Result<ApplicationRuntime, RuntimeError> {
    from_config(entry, &NativeRuntimeConfig::default())
}

pub fn from_config(
    entry: &Path,
    settings: &NativeRuntimeConfig,
) -> Result<ApplicationRuntime, RuntimeError> {
    if !settings.enabled {
        return Err(RuntimeError::Bootstrap(BootstrapError::Disabled));
    }
    let entry = entry
        .canonicalize()
        .map_err(|error| RuntimeError::Bootstrap(BootstrapError::EntryIo(error.kind())))?;
    let rustc = resolve_rustc(settings.rustc_path.as_deref()).map_err(RuntimeError::Bootstrap)?;
    let parent = entry
        .parent()
        .ok_or(RuntimeError::Bootstrap(BootstrapError::MissingAppParent))?;
    let cache_root = settings
        .cache_root
        .clone()
        .or_else(|| env::var_os("VELRAN_NATIVE_CACHE").map(PathBuf::from))
        .unwrap_or_else(|| parent.join(".velran-native-cache"));
    let cache = NativeBuildCache::open(cache_root)
        .map_err(|_| RuntimeError::Bootstrap(BootstrapError::Cache))?;
    let configured_features = settings
        .cpu_features
        .clone()
        .unwrap_or_else(|| env::var("VELRAN_RUSTC_CPU_FEATURES").unwrap_or_default());
    let probe_key =
        RustcProbeKey::from_path(&rustc, settings.target_cpu.as_deref(), &configured_features)
            .map_err(|error| RuntimeError::Bootstrap(BootstrapError::RustcIo(error.kind())))?;
    let identity = match toolchain_identity_cache::load(cache.root(), &probe_key)
        .map_err(|error| RuntimeError::Bootstrap(BootstrapError::RustcIo(error.kind())))?
    {
        Some(identity) => identity,
        None => {
            let identity = rustc_identity(
                &rustc,
                settings.target_cpu.as_deref(),
                Some(&configured_features),
            )
            .map_err(RuntimeError::Bootstrap)?;
            toolchain_identity_cache::store(cache.root(), &probe_key, &identity)
                .map_err(|error| RuntimeError::Bootstrap(BootstrapError::RustcIo(error.kind())))?;
            identity
        }
    };
    let config = RustcConfig::new(rustc, settings.optimization.rustc_profile(), identity)
        .map_err(|_| RuntimeError::Bootstrap(BootstrapError::BuildConfig))?
        .with_repro_diagnostics(settings.debug_rustc_repro);
    let builder = IncrementalBuilder::new(entry, config, cache)
        .map_err(|_| RuntimeError::Bootstrap(BootstrapError::Incremental))?;
    let mut runtime = ApplicationRuntime::new(builder);
    runtime.refresh()?;
    if settings.required && runtime.native_shard_count()? == 0 {
        return Err(RuntimeError::Bootstrap(BootstrapError::NoNativeShards));
    }
    Ok(runtime)
}

fn resolve_rustc(configured: Option<&Path>) -> Result<PathBuf, BootstrapError> {
    if let Some(path) = configured
        .map(Path::to_path_buf)
        .or_else(|| env::var_os("RUSTC").map(PathBuf::from))
    {
        if !path.is_absolute() {
            return Err(BootstrapError::RustcPathNotAbsolute);
        }
        return canonicalize_rustc_path(&path);
    }
    let path = env::var_os("PATH").ok_or(BootstrapError::RustcNotFound)?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(format!("rustc{}", env::consts::EXE_SUFFIX));
        if candidate.is_file() {
            return canonicalize_rustc_path(&candidate);
        }
    }
    Err(BootstrapError::RustcNotFound)
}

fn canonicalize_rustc_path(path: &Path) -> Result<PathBuf, BootstrapError> {
    let canonical = path
        .canonicalize()
        .map_err(|error| BootstrapError::RustcIo(error.kind()))?;
    let canonical_name = canonical.file_name().and_then(|name| name.to_str());
    let requested_name = path.file_name().and_then(|name| name.to_str());
    let rustup_name = format!("rustup{}", env::consts::EXE_SUFFIX);
    let rustc_name = format!("rustc{}", env::consts::EXE_SUFFIX);

    // rustup installs `rustc` as a proxy symlink. Executing the canonical
    // `rustup` path changes argv[0] and no longer selects rustc proxy mode.
    // Resolve that proxy through its sysroot to the immutable toolchain rustc
    // binary instead; this also makes the identity-cache key track toolchain
    // changes rather than only the rustup launcher binary.
    if canonical_name == Some(rustup_name.as_str()) && requested_name == Some(rustc_name.as_str()) {
        let output = Command::new(path)
            .args(["--print", "sysroot"])
            .output()
            .map_err(|error| BootstrapError::RustcIo(error.kind()))?;
        if !output.status.success() {
            return Err(BootstrapError::RustcFailed);
        }
        let sysroot =
            String::from_utf8(output.stdout).map_err(|_| BootstrapError::RustcOutputInvalid)?;
        let toolchain_rustc = PathBuf::from(sysroot.trim()).join("bin").join(rustc_name);
        return toolchain_rustc
            .canonicalize()
            .map_err(|error| BootstrapError::RustcIo(error.kind()));
    }
    Ok(canonical)
}

fn rustc_identity(
    rustc: &Path,
    target_cpu: Option<&str>,
    configured_cpu_features: Option<&str>,
) -> Result<ToolchainIdentity, BootstrapError> {
    let output = Command::new(rustc)
        .arg("-vV")
        .output()
        .map_err(|e| BootstrapError::RustcIo(e.kind()))?;
    if !output.status.success() {
        return Err(BootstrapError::RustcFailed);
    }
    let text = String::from_utf8(output.stdout).map_err(|_| BootstrapError::RustcOutputInvalid)?;
    let rustc_version = text.lines().next().unwrap_or_default().trim().to_owned();
    let target_triple = text
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap_or_default()
        .trim()
        .to_owned();
    if rustc_version.is_empty() || target_triple.is_empty() {
        return Err(BootstrapError::RustcIdentityIncomplete);
    }
    let configured_features = configured_cpu_features
        .map(str::to_owned)
        .unwrap_or_else(|| env::var("VELRAN_RUSTC_CPU_FEATURES").unwrap_or_default());
    let effective_features = if target_cpu.is_some() {
        detect_target_features(rustc, target_cpu, &configured_features)?
    } else {
        configured_features
    };
    Ok(ToolchainIdentity {
        rustc_version,
        target_triple,
        target_cpu: target_cpu.unwrap_or_default().to_owned(),
        cpu_features: effective_features,
    })
}

fn detect_target_features(
    rustc: &Path,
    target_cpu: Option<&str>,
    configured: &str,
) -> Result<String, BootstrapError> {
    let mut command = Command::new(rustc);
    command.args(["--print", "cfg"]);
    if let Some(cpu) = target_cpu {
        command.args(["-C", &format!("target-cpu={cpu}")]);
    }
    if !configured.is_empty() {
        command.args(["-C", &format!("target-feature={configured}")]);
    }
    let output = command
        .output()
        .map_err(|e| BootstrapError::RustcIo(e.kind()))?;
    if !output.status.success() {
        return Err(BootstrapError::RustcFailed);
    }
    let text = String::from_utf8(output.stdout).map_err(|_| BootstrapError::RustcOutputInvalid)?;
    let mut features = text
        .lines()
        .filter_map(|line| {
            line.strip_prefix("target_feature=\"")
                .and_then(|v| v.strip_suffix('\"'))
        })
        .collect::<Vec<_>>();
    features.sort_unstable();
    Ok(features.join(","))
}
