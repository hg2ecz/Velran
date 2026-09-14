#![forbid(unsafe_code)]

use std::fmt;
use std::io;
use std::process::ExitStatus;

use crate::artifact_format::IntegrityError;
use crate::build_cache::CacheError;
use compiler::codegen::CodegenError;

#[derive(Debug)]
pub enum BuildError {
    RustcPathMustBeAbsolute,
    IncompleteToolchainIdentity,
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
    },
    RustcFailed {
        stage: &'static str,
        status: ExitStatus,
        diagnostics: String,
    },
    Integrity(IntegrityError),
    Cache(CacheError),
    UnsupportedLowering(CodegenError),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RustcPathMustBeAbsolute => f.write_str("rustc path must be absolute"),
            Self::IncompleteToolchainIdentity => {
                f.write_str("rustc cache identity requires version and target triple")
            }
            Self::Io { operation, kind } => write!(f, "{operation} failed: {kind}"),
            Self::RustcFailed {
                stage,
                status,
                diagnostics,
            } => match diagnostics.lines().find(|line| !line.trim().is_empty()) {
                Some(first) => write!(f, "rustc {stage} failed with {status}: {first}"),
                None => write!(f, "rustc {stage} failed with {status}"),
            },
            Self::Integrity(error) => write!(f, "artifact integrity failed: {error}"),
            Self::Cache(error) => write!(f, "native build cache failed: {error}"),
            Self::UnsupportedLowering(error) => write!(f, "native lowering unsupported: {error}"),
        }
    }
}
impl std::error::Error for BuildError {}

impl BuildError {
    pub fn rustc_diagnostics(&self) -> Option<&str> {
        match self {
            Self::RustcFailed { diagnostics, .. } => Some(diagnostics.as_str()),
            _ => None,
        }
    }
}

pub(crate) fn io_error(operation: &'static str, error: io::Error) -> BuildError {
    BuildError::Io {
        operation,
        kind: error.kind(),
    }
}
