#![forbid(unsafe_code)]

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum IntegrityError {
    Io(io::ErrorKind),
    SizeMismatch { expected: u64, actual: u64 },
    HashMismatch,
    AbiMismatch { expected: u32, actual: u32 },
    LanguageVersionMismatch,
    SecurityPolicyVersionMismatch,
    ExecutableIrVersionMismatch,
    CodegenVersionMismatch,
    ContractFingerprintMismatch,
    ShardMismatch,
    ShardContentMismatch,
    ManifestVersionMismatch { expected: u16, actual: u16 },
}

impl fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(kind) => write!(f, "artifact I/O failed: {kind}"),
            Self::SizeMismatch { expected, actual } => {
                write!(
                    f,
                    "artifact size mismatch: expected {expected}, got {actual}"
                )
            }
            Self::HashMismatch => f.write_str("artifact SHA-256 mismatch"),
            Self::AbiMismatch { expected, actual } => {
                write!(f, "runtime ABI mismatch: expected {expected}, got {actual}")
            }
            Self::LanguageVersionMismatch => f.write_str("language version mismatch"),
            Self::SecurityPolicyVersionMismatch => f.write_str("security policy version mismatch"),
            Self::ExecutableIrVersionMismatch => f.write_str("executable IR version mismatch"),
            Self::CodegenVersionMismatch => f.write_str("codegen version mismatch"),
            Self::ContractFingerprintMismatch => {
                f.write_str("runtime contract fingerprint mismatch")
            }
            Self::ShardMismatch => f.write_str("shard identity mismatch"),
            Self::ShardContentMismatch => {
                f.write_str("shard generated-source fingerprint mismatch")
            }
            Self::ManifestVersionMismatch { expected, actual } => write!(
                f,
                "artifact manifest version mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for IntegrityError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManifestDecodeError;
