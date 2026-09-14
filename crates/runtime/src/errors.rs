use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceProfileError {
    ZeroDefaultLimit,
    InvalidName(String),
    ZeroNamedLimit(String),
}

impl fmt::Display for ResourceProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDefaultLimit => write!(f, "default resource profile limits must be non-zero"),
            Self::InvalidName(name) => write!(f, "invalid resource profile name `{name}`"),
            Self::ZeroNamedLimit(name) => {
                write!(f, "resource profile `{name}` limits must be non-zero")
            }
        }
    }
}

impl Error for ResourceProfileError {}
