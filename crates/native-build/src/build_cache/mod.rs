#![forbid(unsafe_code)]

mod cleanup;
mod filesystem;

use crate::artifact_format::{ArtifactManifest, IntegrityError};
use compiler::codegen::CODEGEN_VERSION;
use executable_ir::shard_planner::VerifiedShard;
use filesystem::{
    create_private_dir, io_error, read_regular_file, reject_symlink, reject_symlink_components,
    verify_owner, verify_private_permissions,
};
use runtime_abi::RUNTIME_ABI_VERSION;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheIdentity {
    pub rustc_version: String,
    pub target_triple: String,
    pub target_cpu: String,
    pub cpu_features: String,
    pub optimization_profile: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheKey(String);
impl CacheKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheHit {
    pub artifact_path: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: ArtifactManifest,
}
#[derive(Debug)]
pub enum CacheError {
    RootMustBeAbsolute,
    RootIsSymlink,
    InsecurePermissions,
    OwnerMismatch,
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
    },
    CorruptManifest,
    Integrity(IntegrityError),
}
impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootMustBeAbsolute => f.write_str("build-cache root must be absolute"),
            Self::RootIsSymlink => f.write_str("build-cache root must not be a symlink"),
            Self::InsecurePermissions => {
                f.write_str("build-cache root must not be group/world accessible")
            }
            Self::OwnerMismatch => {
                f.write_str("build-cache root must be owned by the current effective user")
            }
            Self::Io { operation, kind } => write!(f, "{operation} failed: {kind}"),
            Self::CorruptManifest => f.write_str("cached artifact manifest is malformed"),
            Self::Integrity(error) => write!(f, "cached artifact integrity failed: {error}"),
        }
    }
}
impl std::error::Error for CacheError {}
#[derive(Debug, Clone)]
pub struct NativeBuildCache {
    root: PathBuf,
}
impl NativeBuildCache {
    pub fn open(root: PathBuf) -> Result<Self, CacheError> {
        if !root.is_absolute() {
            return Err(CacheError::RootMustBeAbsolute);
        }
        create_private_dir(&root)?;
        reject_symlink_components(&root)?;
        let metadata =
            fs::symlink_metadata(&root).map_err(|e| io_error("stat build-cache root", e))?;
        if metadata.file_type().is_symlink() {
            return Err(CacheError::RootIsSymlink);
        }
        verify_private_permissions(&metadata)?;
        verify_owner(&metadata)?;
        cleanup::abandoned_staging(&root)?;
        Ok(Self { root })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn key_for(&self, shard: &VerifiedShard, identity: &CacheIdentity) -> CacheKey {
        let mut bytes = Vec::new();
        field(
            &mut bytes,
            "format",
            "velran-native-cache-v4-runtime-contract",
        );
        field(&mut bytes, "shard", shard.id().as_str());
        field(&mut bytes, "language", shard.language_version());
        field(&mut bytes, "security", shard.security_policy_version());
        field(
            &mut bytes,
            "eir",
            &shard.executable_ir_version().to_string(),
        );
        field(&mut bytes, "abi", &RUNTIME_ABI_VERSION.to_string());
        field(&mut bytes, "codegen", CODEGEN_VERSION);
        field(&mut bytes, "rustc", &identity.rustc_version);
        field(&mut bytes, "target", &identity.target_triple);
        field(&mut bytes, "target_cpu", &identity.target_cpu);
        field(&mut bytes, "cpu", &identity.cpu_features);
        field(&mut bytes, "opt", &identity.optimization_profile);
        field(&mut bytes, "interface", shard.interface_sha256());
        field(&mut bytes, "implementation", shard.implementation_sha256());
        CacheKey(format!("{:x}", Sha256::digest(bytes)))
    }
    pub fn lookup(
        &self,
        key: &CacheKey,
        shard: &VerifiedShard,
    ) -> Result<Option<CacheHit>, CacheError> {
        let dir = self.entry_dir(key);
        match fs::symlink_metadata(&dir) {
            Ok(meta) => {
                if meta.file_type().is_symlink() || !meta.is_dir() {
                    return Err(CacheError::RootIsSymlink);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io_error("stat cache entry", error)),
        }
        let manifest_path = dir.join("artifact.manifest");
        let manifest_text = read_regular_file(&manifest_path, "read cache manifest")?;
        let manifest = ArtifactManifest::decode(
            std::str::from_utf8(&manifest_text).map_err(|_| CacheError::CorruptManifest)?,
        )
        .map_err(|_| CacheError::CorruptManifest)?;
        let artifact_path = dir.join(dynamic_library_name("artifact"));
        reject_symlink(&artifact_path)?;
        manifest
            .verify_for(shard, &artifact_path)
            .map_err(CacheError::Integrity)?;
        Ok(Some(CacheHit {
            artifact_path,
            manifest_path,
            manifest,
        }))
    }
    pub fn store(
        &self,
        key: &CacheKey,
        shard: &VerifiedShard,
        source_artifact: &Path,
        source_manifest: &ArtifactManifest,
    ) -> Result<CacheHit, CacheError> {
        if let Some(hit) = self.lookup(key, shard)? {
            return Ok(hit);
        }
        let staging = self.root.join(format!(
            ".{}.{}.{}.tmp",
            key.as_str(),
            std::process::id(),
            next_nonce()
        ));
        create_private_dir(&staging)?;
        let artifact_path = staging.join(dynamic_library_name("artifact"));
        fs::copy(source_artifact, &artifact_path)
            .map_err(|e| io_error("copy artifact into cache", e))?;
        source_manifest
            .verify_for(shard, &artifact_path)
            .map_err(CacheError::Integrity)?;
        let manifest_path = staging.join("artifact.manifest");
        fs::write(&manifest_path, source_manifest.encode())
            .map_err(|e| io_error("write cache manifest", e))?;
        let final_dir = self.entry_dir(key);
        match fs::rename(&staging, &final_dir) {
            Ok(()) => {}
            Err(error) if final_dir.exists() => {
                fs::remove_dir_all(&staging)
                    .map_err(|e| io_error("remove duplicate cache staging", e))?;
                let _ = error;
            }
            Err(error) => return Err(io_error("publish cache entry", error)),
        }
        self.lookup(key, shard)?.ok_or(CacheError::CorruptManifest)
    }
    fn entry_dir(&self, key: &CacheKey) -> PathBuf {
        self.root.join(key.as_str())
    }
}
fn field(out: &mut Vec<u8>, name: &str, value: &str) {
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    out.extend_from_slice(value.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(value.as_bytes());
    out.push(0xff);
}

fn next_nonce() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn dynamic_library_name(stem: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn field_encoding_is_length_delimited() {
        let mut a = Vec::new();
        field(&mut a, "x", "ab");
        field(&mut a, "y", "c");
        let mut b = Vec::new();
        field(&mut b, "x", "a");
        field(&mut b, "y", "bc");
        assert_ne!(a, b);
    }
}
