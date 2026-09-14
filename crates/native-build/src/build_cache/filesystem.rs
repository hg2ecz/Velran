#![forbid(unsafe_code)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::CacheError;

pub(super) fn reject_symlink_components(path: &Path) -> Result<(), CacheError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if current == Path::new("/") {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(CacheError::RootIsSymlink);
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("inspect cache path component", error)),
        }
    }
    Ok(())
}

pub(super) fn reject_symlink(path: &Path) -> Result<(), CacheError> {
    let metadata = fs::symlink_metadata(path).map_err(|e| io_error("stat cached artifact", e))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CacheError::RootIsSymlink);
    }
    Ok(())
}

pub(super) fn read_regular_file(
    path: &Path,
    operation: &'static str,
) -> Result<Vec<u8>, CacheError> {
    reject_symlink(path)?;
    fs::read(path).map_err(|e| io_error(operation, e))
}

pub(super) fn io_error(operation: &'static str, error: io::Error) -> CacheError {
    CacheError::Io {
        operation,
        kind: error.kind(),
    }
}

#[cfg(unix)]
pub(super) fn create_private_dir(path: &Path) -> Result<(), CacheError> {
    use std::os::unix::fs::DirBuilderExt;
    if path.exists() {
        return Ok(());
    }
    let mut builder = fs::DirBuilder::new();
    builder.recursive(false).mode(0o700);
    builder
        .create(path)
        .map_err(|e| io_error("create private cache directory", e))
}

#[cfg(not(unix))]
pub(super) fn create_private_dir(path: &Path) -> Result<(), CacheError> {
    fs::create_dir(path).map_err(|e| io_error("create private cache directory", e))
}

#[cfg(unix)]
pub(super) fn verify_private_permissions(metadata: &fs::Metadata) -> Result<(), CacheError> {
    use std::os::unix::fs::PermissionsExt;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(CacheError::InsecurePermissions);
    }
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn verify_private_permissions(_: &fs::Metadata) -> Result<(), CacheError> {
    Ok(())
}

#[cfg(target_os = "linux")]
pub(super) fn verify_owner(metadata: &fs::Metadata) -> Result<(), CacheError> {
    use std::os::unix::fs::MetadataExt;
    let status =
        fs::read_to_string("/proc/self/status").map_err(|e| io_error("read effective uid", e))?;
    let uid_line = status
        .lines()
        .find(|line| line.starts_with("Uid:"))
        .ok_or(CacheError::OwnerMismatch)?;
    let effective = uid_line
        .split_whitespace()
        .nth(2)
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or(CacheError::OwnerMismatch)?;
    if metadata.uid() != effective {
        return Err(CacheError::OwnerMismatch);
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub(super) fn verify_owner(_: &fs::Metadata) -> Result<(), CacheError> {
    Ok(())
}
