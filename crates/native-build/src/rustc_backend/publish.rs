#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use executable_ir::shard_planner::VerifiedShard;

use crate::artifact_format::{ArtifactManifest, IntegrityError};

use super::{BuildError, io_error};

pub(super) fn publish_immutable(
    staged_path: &Path,
    final_path: &Path,
    shard: &VerifiedShard,
    crate_name: &str,
    staged: &ArtifactManifest,
) -> Result<(), BuildError> {
    if final_path.exists() {
        let existing = ArtifactManifest::from_artifact(shard, crate_name.to_owned(), final_path)
            .map_err(BuildError::Integrity)?;
        if existing.artifact_sha256 != staged.artifact_sha256 {
            return Err(BuildError::Integrity(IntegrityError::HashMismatch));
        }
        fs::remove_file(staged_path)
            .map_err(|error| io_error("remove duplicate staged artifact", error))?;
    } else {
        fs::rename(staged_path, final_path)
            .map_err(|error| io_error("atomically publish artifact", error))?;
    }
    Ok(())
}

pub(super) fn dynamic_library_name(stem: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), BuildError> {
    let temp = path.with_extension("manifest.tmp");
    fs::write(&temp, bytes).map_err(|error| io_error("write staged manifest", error))?;
    fs::rename(&temp, path).map_err(|error| io_error("atomically publish manifest", error))?;
    Ok(())
}
