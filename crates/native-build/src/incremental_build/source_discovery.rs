#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use super::IncrementalBuildError;
use super::source_tracking::SourceState;

pub(super) fn discover_source_paths(
    roots: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<PathBuf>, IncrementalBuildError> {
    let mut pending: Vec<PathBuf> = roots.iter().cloned().collect();
    let mut paths = BTreeSet::new();
    while let Some(dir) = pending.pop() {
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .map_err(|error| io_error("scan source directory", error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| io_error("scan source directory entry", error))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let meta = fs::symlink_metadata(&path)
                .map_err(|error| io_error("stat source tree entry", error))?;
            if meta.file_type().is_symlink() {
                if path.extension().and_then(|v| v.to_str()) == Some("vrn") {
                    return Err(IncrementalBuildError::SourceNotRegular(path));
                }
                continue;
            }
            if meta.is_dir() {
                pending.push(path);
            } else if meta.is_file()
                && (path.extension().and_then(|v| v.to_str()) == Some("vrn")
                    || path.file_name().and_then(|v| v.to_str()) == Some("velran.toml"))
            {
                paths.insert(
                    path.canonicalize()
                        .map_err(|error| io_error("canonicalize source", error))?,
                );
            }
        }
    }
    Ok(paths)
}

pub(super) fn changed_source_paths(
    previous: Option<&BTreeMap<PathBuf, SourceState>>,
    current: &BTreeMap<PathBuf, SourceState>,
) -> Vec<PathBuf> {
    let mut changed = BTreeSet::new();
    for (path, state) in current {
        if previous
            .and_then(|sources| sources.get(path))
            .map(|old| &old.sha256)
            != Some(&state.sha256)
        {
            changed.insert(path.clone());
        }
    }
    if let Some(previous) = previous {
        for path in previous.keys() {
            if !current.contains_key(path) {
                changed.insert(path.clone());
            }
        }
    }
    changed.into_iter().collect()
}

fn io_error(operation: &'static str, error: std::io::Error) -> IncrementalBuildError {
    IncrementalBuildError::Io {
        operation,
        kind: error.kind(),
    }
}
