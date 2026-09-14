use native_build::rustc_backend::ToolchainIdentity;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const IDENTITY_FILE: &str = ".rustc-identity-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RustcProbeKey {
    rustc_path: PathBuf,
    rustc_len: u64,
    rustc_modified_secs: u64,
    rustc_modified_nanos: u32,
    target_cpu: String,
    configured_features: String,
}

impl RustcProbeKey {
    pub(super) fn from_path(
        rustc_path: &Path,
        target_cpu: Option<&str>,
        configured_features: &str,
    ) -> io::Result<Self> {
        let metadata = fs::metadata(rustc_path)?;
        let modified = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "rustc modification time predates UNIX epoch",
                )
            })?;
        Ok(Self {
            rustc_path: rustc_path.to_path_buf(),
            rustc_len: metadata.len(),
            rustc_modified_secs: modified.as_secs(),
            rustc_modified_nanos: modified.subsec_nanos(),
            target_cpu: target_cpu.unwrap_or_default().to_owned(),
            configured_features: configured_features.to_owned(),
        })
    }
}

pub(super) fn load(
    cache_root: &Path,
    key: &RustcProbeKey,
) -> io::Result<Option<ToolchainIdentity>> {
    let path = cache_root.join(IDENTITY_FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid rustc identity cache utf-8",
        )
    })?;
    let mut values = std::collections::BTreeMap::new();
    for line in text.lines() {
        let Some((name, value)) = line.split_once('=') else {
            return Ok(None);
        };
        if values.insert(name, value).is_some() {
            return Ok(None);
        }
    }
    if values.len() != 9 {
        return Ok(None);
    }
    let get = |name: &str| values.get(name).copied();
    if get("rustc_path") != Some(key.rustc_path.to_string_lossy().as_ref())
        || get("rustc_len").and_then(|v| v.parse::<u64>().ok()) != Some(key.rustc_len)
        || get("rustc_modified_secs").and_then(|v| v.parse::<u64>().ok())
            != Some(key.rustc_modified_secs)
        || get("rustc_modified_nanos").and_then(|v| v.parse::<u32>().ok())
            != Some(key.rustc_modified_nanos)
        || get("target_cpu") != Some(key.target_cpu.as_str())
        || get("configured_features") != Some(key.configured_features.as_str())
    {
        return Ok(None);
    }
    let rustc_version = get("rustc_version").unwrap_or_default().to_owned();
    let target_triple = get("target_triple").unwrap_or_default().to_owned();
    let cpu_features = get("effective_features").unwrap_or_default().to_owned();
    if rustc_version.is_empty() || target_triple.is_empty() {
        return Ok(None);
    }
    Ok(Some(ToolchainIdentity {
        rustc_version,
        target_triple,
        target_cpu: key.target_cpu.clone(),
        cpu_features,
    }))
}

pub(super) fn store(
    cache_root: &Path,
    key: &RustcProbeKey,
    identity: &ToolchainIdentity,
) -> io::Result<()> {
    let final_path = cache_root.join(IDENTITY_FILE);
    let staging_path = cache_root.join(format!("{IDENTITY_FILE}.{}.tmp", std::process::id()));
    let text = format!(
        "rustc_path={}\nrustc_len={}\nrustc_modified_secs={}\nrustc_modified_nanos={}\ntarget_cpu={}\nconfigured_features={}\nrustc_version={}\ntarget_triple={}\neffective_features={}\n",
        key.rustc_path.to_string_lossy(),
        key.rustc_len,
        key.rustc_modified_secs,
        key.rustc_modified_nanos,
        key.target_cpu,
        key.configured_features,
        identity.rustc_version,
        identity.target_triple,
        identity.cpu_features,
    );
    fs::write(&staging_path, text)?;
    fs::rename(&staging_path, &final_path)?;
    Ok(())
}
