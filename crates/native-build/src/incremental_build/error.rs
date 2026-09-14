#![forbid(unsafe_code)]

use std::fmt;
use std::io;
use std::path::PathBuf;

use compiler::VerifiedCompileError;

use crate::rustc_backend::BuildError;

#[derive(Debug)]
pub enum IncrementalBuildError {
    EntryMustBeAbsolute,
    SourceNotRegular(PathBuf),
    SourceTooLarge(PathBuf),
    SourceChangedDuringHash(PathBuf),
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
    },
    Compile(VerifiedCompileError),
    Native(BuildError),
    UnsupportedNativeShard(String),
}

impl fmt::Display for IncrementalBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryMustBeAbsolute => {
                f.write_str("incremental build entrypoint must be absolute")
            }
            Self::SourceNotRegular(path) => write!(
                f,
                "tracked source is not a regular non-symlink file: {}",
                path.display()
            ),
            Self::SourceTooLarge(path) => write!(
                f,
                "tracked source exceeds the per-module size limit: {}",
                path.display()
            ),
            Self::SourceChangedDuringHash(path) => write!(
                f,
                "tracked source changed while hashing: {}",
                path.display()
            ),
            Self::Io { operation, kind } => write!(f, "{operation} failed: {kind}"),
            Self::Compile(error) => write!(f, "incremental frontend compile failed: {error:?}"),
            Self::Native(error) => write!(f, "incremental native build failed: {error}"),
            Self::UnsupportedNativeShard(shard) => write!(
                f,
                "shard `{shard}` cannot be lowered to the native cdylib backend"
            ),
        }
    }
}
impl std::error::Error for IncrementalBuildError {}

impl IncrementalBuildError {
    pub fn rustc_diagnostics(&self) -> Option<&str> {
        match self {
            Self::Native(error) => error.rustc_diagnostics(),
            _ => None,
        }
    }
}
