use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum RatePolicyConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Syntax {
        path: PathBuf,
        line: usize,
        message: String,
    },
    MissingField {
        policy: String,
        field: &'static str,
    },
    InvalidNumber {
        policy: String,
        field: &'static str,
        source: std::num::ParseIntError,
    },
    InvalidLimits {
        policy: String,
    },
    UnknownScope {
        policy: String,
        scope: String,
    },
    UnknownKey {
        policy: String,
        key: String,
    },
    UnknownRoutePolicy {
        route: String,
        policy: String,
    },
    PublicUserScopedPolicy {
        route: String,
        policy: String,
    },
}

impl fmt::Display for RatePolicyConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(
                f,
                "failed to read rate policy file `{}`: {source}",
                path.display()
            ),
            Self::Syntax {
                path,
                line,
                message,
            } => write!(f, "{}:{line} {message}", path.display()),
            Self::MissingField { policy, field } => write!(f, "policy `{policy}` missing {field}"),
            Self::InvalidNumber {
                policy,
                field,
                source,
            } => write!(f, "policy `{policy}` has invalid {field}: {source}"),
            Self::InvalidLimits { policy } => {
                write!(f, "policy `{policy}` has invalid limit/window_secs")
            }
            Self::UnknownScope { policy, scope } => {
                write!(f, "policy `{policy}` unknown scope `{scope}`")
            }
            Self::UnknownKey { policy, key } => write!(f, "policy `{policy}` unknown key `{key}`"),
            Self::UnknownRoutePolicy { route, policy } => {
                write!(f, "route `{route}` requests unknown rate policy `{policy}`")
            }
            Self::PublicUserScopedPolicy { route, policy } => write!(
                f,
                "public route `{route}` cannot use user-scoped rate policy `{policy}`"
            ),
        }
    }
}

impl Error for RatePolicyConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::InvalidNumber { source, .. } => Some(source),
            _ => None,
        }
    }
}
