use native_build::artifact_format::IntegrityError;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum LoadError {
    ArtifactMustBeAbsolute,
    ArtifactIsSymlink,
    Io(io::ErrorKind),
    Integrity(IntegrityError),
    UnsupportedPlatform,
    DynamicLoad,
    MissingAbiSymbol,
    MissingContractSymbol,
    MissingInvokeSymbol,
    MalformedInput,
    MalformedResult,
    AbiMismatch { expected: u32, actual: u32 },
    ContractMismatch { expected: u64, actual: u64 },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArtifactMustBeAbsolute => f.write_str("native artifact path must be absolute"),
            Self::ArtifactIsSymlink => f.write_str("native artifact must not be a symlink"),
            Self::Io(kind) => write!(f, "native artifact I/O failed: {kind}"),
            Self::Integrity(error) => write!(f, "native artifact integrity failed: {error}"),
            Self::UnsupportedPlatform => {
                f.write_str("native artifact loading is unsupported on this platform")
            }
            Self::DynamicLoad => f.write_str("dynamic library load failed"),
            Self::MissingAbiSymbol => f.write_str("native artifact is missing ABI version symbol"),
            Self::MissingContractSymbol => {
                f.write_str("native artifact is missing runtime contract fingerprint symbol")
            }
            Self::MissingInvokeSymbol => f.write_str("native artifact is missing invoke symbol"),
            Self::MalformedInput => f.write_str("native request input exceeds ABI limits"),
            Self::MalformedResult => f.write_str("native artifact returned malformed ABI result"),
            Self::AbiMismatch { expected, actual } => {
                write!(
                    f,
                    "loaded native ABI mismatch: expected {expected}, got {actual}"
                )
            }
            Self::ContractMismatch { expected, actual } => write!(
                f,
                "loaded native contract mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for LoadError {}
