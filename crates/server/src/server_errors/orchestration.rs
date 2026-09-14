use std::error::Error;
use std::fmt;
use std::io;

use super::{
    AuthSetupError, BackendSupportError, RatePolicyConfigError, ResourceProfileConfigError,
    TlsConfigError,
};

#[derive(Debug)]
pub(crate) enum ConnectionError {
    Io(io::Error),
    Auth(auth::AuthError),
    HostingLockPoisoned,
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "connection I/O error: {source}"),
            Self::Auth(source) => write!(f, "authentication error: {source}"),
            Self::HostingLockPoisoned => f.write_str("hosting runtime lock poisoned"),
        }
    }
}
impl Error for ConnectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            Self::Auth(source) => Some(source),
            Self::HostingLockPoisoned => None,
        }
    }
}
impl From<io::Error> for ConnectionError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<auth::AuthError> for ConnectionError {
    fn from(value: auth::AuthError) -> Self {
        Self::Auth(value)
    }
}

#[derive(Debug)]
pub(crate) enum StartupError {
    Invalid(String),
    Io(io::Error),
    Address(ipnet::AddrParseError),
    Backend(BackendSupportError),
    Data(data::DataError),
    RatePolicy(RatePolicyConfigError),
    ResourceProfile(ResourceProfileConfigError),
    AuthSetup(AuthSetupError),
    Auth(auth::AuthError),
    Tls(TlsConfigError),
}
impl StartupError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    pub(crate) fn compiler_error(&self) -> Option<&compiler::CompileError> {
        match self {
            Self::Backend(source) => source.compiler_error(),
            _ => None,
        }
    }

    pub(crate) fn rustc_diagnostics(&self) -> Option<&str> {
        match self {
            Self::Backend(source) => source.rustc_diagnostics(),
            _ => None,
        }
    }
}
impl fmt::Display for StartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Io(source) => write!(f, "I/O error: {source}"),
            Self::Address(source) => write!(f, "invalid network address: {source}"),
            Self::Backend(source) => write!(f, "{source}"),
            Self::Data(source) => write!(f, "data backend error: {source}"),
            Self::RatePolicy(source) => write!(f, "{source}"),
            Self::ResourceProfile(source) => write!(f, "{source}"),
            Self::AuthSetup(source) => write!(f, "{source}"),
            Self::Auth(source) => write!(f, "authentication error: {source}"),
            Self::Tls(source) => write!(f, "{source}"),
        }
    }
}
impl Error for StartupError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Invalid(_) => None,
            Self::Io(source) => Some(source),
            Self::Address(source) => Some(source),
            Self::Backend(source) => Some(source),
            Self::Data(source) => Some(source),
            Self::RatePolicy(source) => Some(source),
            Self::ResourceProfile(source) => Some(source),
            Self::AuthSetup(source) => Some(source),
            Self::Auth(source) => Some(source),
            Self::Tls(source) => Some(source),
        }
    }
}
impl From<io::Error> for StartupError {
    fn from(v: io::Error) -> Self {
        Self::Io(v)
    }
}
impl From<ipnet::AddrParseError> for StartupError {
    fn from(v: ipnet::AddrParseError) -> Self {
        Self::Address(v)
    }
}
impl From<BackendSupportError> for StartupError {
    fn from(v: BackendSupportError) -> Self {
        Self::Backend(v)
    }
}
impl From<data::DataError> for StartupError {
    fn from(v: data::DataError) -> Self {
        Self::Data(v)
    }
}
impl From<RatePolicyConfigError> for StartupError {
    fn from(v: RatePolicyConfigError) -> Self {
        Self::RatePolicy(v)
    }
}
impl From<ResourceProfileConfigError> for StartupError {
    fn from(v: ResourceProfileConfigError) -> Self {
        Self::ResourceProfile(v)
    }
}
impl From<AuthSetupError> for StartupError {
    fn from(v: AuthSetupError) -> Self {
        Self::AuthSetup(v)
    }
}
impl From<auth::AuthError> for StartupError {
    fn from(v: auth::AuthError) -> Self {
        Self::Auth(v)
    }
}
impl From<TlsConfigError> for StartupError {
    fn from(v: TlsConfigError) -> Self {
        Self::Tls(v)
    }
}
