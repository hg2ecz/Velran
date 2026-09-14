use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum AuthSetupError {
    MissingLdapField(&'static str),
    LdapValidation(auth::AuthError),
    ReadFile {
        path: PathBuf,
        source: io::Error,
    },
    InvalidLine {
        path: PathBuf,
        line: usize,
        message: &'static str,
    },
    InvalidUsername {
        path: PathBuf,
        line: usize,
    },
    DuplicateUsername {
        path: PathBuf,
        line: usize,
    },
    InvalidTotpHex {
        path: PathBuf,
        line: usize,
    },
    TotpSecretTooShort {
        path: PathBuf,
        line: usize,
    },
    InvalidRole {
        path: PathBuf,
        line: usize,
        role: String,
    },
    InvalidTenant {
        path: PathBuf,
        line: usize,
        tenant: String,
    },
}

impl fmt::Display for AuthSetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLdapField(field) => write!(f, "LDAP config missing {field}"),
            Self::LdapValidation(source) => write!(f, "invalid LDAP configuration: {source}"),
            Self::ReadFile { path, source } => {
                write!(f, "failed to read auth file `{}`: {source}", path.display())
            }
            Self::InvalidLine {
                path,
                line,
                message,
            } => write!(f, "{}:{line} {message}", path.display()),
            Self::InvalidUsername { path, line } => {
                write!(f, "{}:{line} invalid username", path.display())
            }
            Self::DuplicateUsername { path, line } => {
                write!(f, "{}:{line} duplicate username", path.display())
            }
            Self::InvalidTotpHex { path, line } => {
                write!(f, "{}:{line} invalid hex TOTP secret", path.display())
            }
            Self::TotpSecretTooShort { path, line } => write!(
                f,
                "{}:{line} TOTP secret must be at least 20 bytes",
                path.display()
            ),
            Self::InvalidRole { path, line, role } => {
                write!(f, "{}:{line} invalid role `{role}`", path.display())
            }
            Self::InvalidTenant { path, line, tenant } => {
                write!(f, "{}:{line} invalid tenant `{tenant}`", path.display())
            }
        }
    }
}

impl Error for AuthSetupError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LdapValidation(source) => Some(source),
            Self::ReadFile { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<auth::AuthError> for AuthSetupError {
    fn from(value: auth::AuthError) -> Self {
        Self::LdapValidation(value)
    }
}
