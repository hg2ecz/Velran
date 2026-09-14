use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub(crate) enum CliValueError {
    MissingValue {
        flag: String,
    },
    InvalidNumber {
        flag: String,
        source: std::num::ParseIntError,
    },
    MustBePositive {
        flag: String,
    },
}

impl fmt::Display for CliValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue { flag } => write!(f, "{flag} requires a number"),
            Self::InvalidNumber { flag, source } => {
                write!(f, "{flag} requires a valid number: {source}")
            }
            Self::MustBePositive { flag } => write!(f, "{flag} must be greater than zero"),
        }
    }
}

impl Error for CliValueError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidNumber { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ReservedPathError {
    value: String,
}

impl ReservedPathError {
    pub(crate) fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl fmt::Display for ReservedPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid reserved endpoint path `{}`", self.value)
    }
}

impl Error for ReservedPathError {}
