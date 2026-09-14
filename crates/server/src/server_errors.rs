use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub(super) struct PublicHostError {
    value: String,
}

impl PublicHostError {
    pub(super) fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl fmt::Display for PublicHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid public host `{}`", self.value)
    }
}

impl Error for PublicHostError {}

#[derive(Debug, Clone, Copy)]
pub(super) enum StaticPrefixError {
    Shape,
    DotSegment,
    UnsafeSegment,
}

impl fmt::Display for StaticPrefixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape => write!(f, "static URL prefix must look like `/assets/`"),
            Self::DotSegment => write!(f, "static URL prefix cannot contain dot segments"),
            Self::UnsafeSegment => write!(
                f,
                "static URL prefix must be a single URL-safe path segment such as `/assets/`"
            ),
        }
    }
}

impl Error for StaticPrefixError {}

#[derive(Debug)]
pub(super) enum ServerConfigError {
    Invalid(String),
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Toml {
        kind: &'static str,
        path: PathBuf,
        source: toml::de::Error,
    },
    PublicHost(PublicHostError),
    StaticPrefix(StaticPrefixError),
}

impl ServerConfigError {
    pub(super) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    pub(super) fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for ServerConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Io {
                operation,
                path,
                source,
            } => write!(f, "failed to {operation} `{}`: {source}", path.display()),
            Self::Toml { kind, path, source } => {
                write!(f, "invalid {kind} config `{}`: {source}", path.display())
            }
            Self::PublicHost(source) => write!(f, "{source}"),
            Self::StaticPrefix(source) => write!(f, "{source}"),
        }
    }
}

impl Error for ServerConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Toml { source, .. } => Some(source),
            Self::PublicHost(source) => Some(source),
            Self::StaticPrefix(source) => Some(source),
            Self::Invalid(_) => None,
        }
    }
}

impl From<PublicHostError> for ServerConfigError {
    fn from(value: PublicHostError) -> Self {
        Self::PublicHost(value)
    }
}

impl From<StaticPrefixError> for ServerConfigError {
    fn from(value: StaticPrefixError) -> Self {
        Self::StaticPrefix(value)
    }
}

#[derive(Debug)]
pub(super) enum TlsConfigError {
    Invalid(String),
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Rustls(String),
}

impl TlsConfigError {
    pub(super) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    pub(super) fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.into(),
            source,
        }
    }

    pub(super) fn rustls(error: impl fmt::Display) -> Self {
        Self::Rustls(error.to_string())
    }
}

impl fmt::Display for TlsConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Io {
                operation,
                path,
                source,
            } => write!(f, "failed to {operation} `{}`: {source}", path.display()),
            Self::Rustls(message) => write!(f, "TLS configuration error: {message}"),
        }
    }
}

impl Error for TlsConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(super) enum SecretFileError {
    Read { path: PathBuf, source: io::Error },
    Empty { path: PathBuf },
}

impl fmt::Display for SecretFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    f,
                    "failed to read secret file `{}`: {source}",
                    path.display()
                )
            }
            Self::Empty { path } => write!(f, "secret file `{}` is empty", path.display()),
        }
    }
}

impl Error for SecretFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Empty { .. } => None,
        }
    }
}

mod auth_setup;
mod backend;
mod cli;
mod cli_value;
mod orchestration;
mod policy_config;
mod resource_profile_config;
mod runtime_boundary;
mod source_reload;

pub(super) use auth_setup::AuthSetupError;
pub(super) use backend::BackendSupportError;
pub(super) use cli::CliParseError;
pub(super) use cli_value::{CliValueError, ReservedPathError};
pub(super) use orchestration::{ConnectionError, StartupError};
pub(super) use policy_config::RatePolicyConfigError;
pub(super) use resource_profile_config::ResourceProfileConfigError;
pub(super) use runtime_boundary::{ClockError, PublicCacheError, UploadRuntimeError};
pub(super) use source_reload::SourceReloadError;
