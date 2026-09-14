use std::fmt;

#[derive(Debug)]
pub enum ObsError {
    Serialization,
}

impl fmt::Display for ObsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization => f.write_str("serialization failed"),
        }
    }
}

impl std::error::Error for ObsError {}
