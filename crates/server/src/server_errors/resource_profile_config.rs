use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum ResourceProfileConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Syntax {
        path: PathBuf,
        line: usize,
        message: String,
    },
    InvalidNumber {
        path: PathBuf,
        line: usize,
        key: String,
        source: std::num::ParseIntError,
    },
    MissingField {
        profile: String,
        field: &'static str,
    },
    ConcurrentOverflow {
        profile: String,
        source: std::num::TryFromIntError,
    },
    ExceedsRequestCeiling {
        profile: String,
    },
    InvalidProfile(runtime::ResourceProfileError),
    UnknownProfileUse {
        file: String,
        line: usize,
        function: String,
        profile: String,
    },
}

impl fmt::Display for ResourceProfileConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(
                f,
                "failed to read resource profile file `{}`: {source}",
                path.display()
            ),
            Self::Syntax {
                path,
                line,
                message,
            } => write!(f, "{}:{line} {message}", path.display()),
            Self::InvalidNumber {
                path,
                line,
                key,
                source,
            } => write!(
                f,
                "{}:{line} invalid value for `{key}`: {source}",
                path.display()
            ),
            Self::MissingField { profile, field } => {
                write!(f, "resource profile `{profile}` missing `{field}`")
            }
            Self::ConcurrentOverflow { profile, source } => write!(
                f,
                "resource profile `{profile}` max_concurrent is too large: {source}"
            ),
            Self::ExceedsRequestCeiling { profile } => write!(
                f,
                "resource profile `{profile}` exceeds request hard ceiling"
            ),
            Self::InvalidProfile(source) => {
                write!(f, "invalid resource profile configuration: {source}")
            }
            Self::UnknownProfileUse {
                file,
                line,
                function,
                profile,
            } => write!(
                f,
                "{file}:{line} function `{function}` requests unknown resource profile `{profile}`"
            ),
        }
    }
}

impl Error for ResourceProfileConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::InvalidNumber { source, .. } => Some(source),
            Self::ConcurrentOverflow { source, .. } => Some(source),
            Self::InvalidProfile(source) => Some(source),
            _ => None,
        }
    }
}

impl From<runtime::ResourceProfileError> for ResourceProfileConfigError {
    fn from(value: runtime::ResourceProfileError) -> Self {
        Self::InvalidProfile(value)
    }
}
