use crate::diagnostics::CompileError;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

pub(crate) struct SourceSnapshot {
    pub(crate) bytes: Vec<u8>,
    pub(crate) sha256: String,
    pub(crate) len: u64,
    pub(crate) modified: Option<SystemTime>,
}

pub(crate) fn read(path: &Path) -> Result<SourceSnapshot, CompileError> {
    let before = regular_metadata(path)?;
    let bytes = fs::read(path)?;
    let after = regular_metadata(path)?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err(CompileError::Syntax(format!(
            "module `{}` changed while being read; retry after the write completes",
            path.display()
        )));
    }
    Ok(SourceSnapshot {
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        len: bytes.len() as u64,
        modified: after.modified().ok(),
        bytes,
    })
}

fn regular_metadata(path: &Path) -> Result<fs::Metadata, CompileError> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(CompileError::Syntax(format!(
            "module `{}` must be a regular non-symlink file",
            path.display()
        )));
    }
    Ok(meta)
}
