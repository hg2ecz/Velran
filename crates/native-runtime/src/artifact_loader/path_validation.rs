use std::path::Path;

use super::LoadError;

pub(super) fn validate(path: &Path) -> Result<(), LoadError> {
    if !path.is_absolute() {
        return Err(LoadError::ArtifactMustBeAbsolute);
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|error| LoadError::Io(error.kind()))?;
    if metadata.file_type().is_symlink() {
        return Err(LoadError::ArtifactIsSymlink);
    }
    Ok(())
}
