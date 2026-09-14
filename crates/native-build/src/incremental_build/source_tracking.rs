use compiler::CompiledSource;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::IncrementalBuildError;

const MAX_TRACKED_SOURCE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceState {
    pub(super) len: u64,
    pub(super) modified: Option<SystemTime>,
    pub(super) sha256: String,
}

pub(super) struct SourceProbe {
    pub(super) metadata_changed: Vec<PathBuf>,
    pub(super) content_changed: Vec<PathBuf>,
    pub(super) refreshed: BTreeMap<PathBuf, SourceState>,
}

pub(super) fn snapshot_compiled_sources(
    sources: &[CompiledSource],
) -> Result<BTreeMap<PathBuf, SourceState>, IncrementalBuildError> {
    Ok(sources
        .iter()
        .map(|source| {
            (
                source.path.clone(),
                SourceState {
                    len: source.len,
                    modified: source.modified,
                    sha256: source.sha256.clone(),
                },
            )
        })
        .collect())
}

pub(super) fn probe_known_sources(
    previous: &BTreeMap<PathBuf, SourceState>,
) -> Result<SourceProbe, IncrementalBuildError> {
    let mut metadata_changed = Vec::new();
    let mut content_changed = Vec::new();
    let mut refreshed = BTreeMap::new();
    for (path, old) in previous {
        let meta = match fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
                return Err(IncrementalBuildError::SourceNotRegular(path.clone()));
            }
            Ok(meta) => meta,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                metadata_changed.push(path.clone());
                content_changed.push(path.clone());
                continue;
            }
            Err(error) => return Err(io_error("stat tracked source", error)),
        };
        let modified = meta.modified().ok();
        if meta.len() == old.len && modified == old.modified {
            continue;
        }
        metadata_changed.push(path.clone());
        let new = hash_source(path)?;
        if new.sha256 != old.sha256 {
            content_changed.push(path.clone());
        }
        refreshed.insert(path.clone(), new);
    }
    Ok(SourceProbe {
        metadata_changed,
        content_changed,
        refreshed,
    })
}

pub(super) fn hash_source(path: &Path) -> Result<SourceState, IncrementalBuildError> {
    let before = regular_metadata(path)?;
    if before.len() > MAX_TRACKED_SOURCE_BYTES {
        return Err(IncrementalBuildError::SourceTooLarge(path.to_path_buf()));
    }
    let mut file = fs::File::open(path).map_err(|error| io_error("open tracked source", error))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 16 * 1024];
    let mut total = 0u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| io_error("hash tracked source", error))?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > MAX_TRACKED_SOURCE_BYTES {
            return Err(IncrementalBuildError::SourceTooLarge(path.to_path_buf()));
        }
        hasher.update(&buffer[..read]);
    }
    let after = regular_metadata(path)?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err(IncrementalBuildError::SourceChangedDuringHash(
            path.to_path_buf(),
        ));
    }
    Ok(SourceState {
        len: after.len(),
        modified: after.modified().ok(),
        sha256: format!("{:x}", hasher.finalize()),
    })
}

fn regular_metadata(path: &Path) -> Result<fs::Metadata, IncrementalBuildError> {
    let meta =
        fs::symlink_metadata(path).map_err(|error| io_error("stat tracked source", error))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(IncrementalBuildError::SourceNotRegular(path.to_path_buf()));
    }
    Ok(meta)
}

fn io_error(operation: &'static str, error: io::Error) -> IncrementalBuildError {
    IncrementalBuildError::Io {
        operation,
        kind: error.kind(),
    }
}
