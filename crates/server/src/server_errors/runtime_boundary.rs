use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;
use std::time::SystemTimeError;

#[derive(Debug)]
pub(crate) enum ClockError {
    BeforeUnixEpoch(SystemTimeError),
}

impl fmt::Display for ClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeforeUnixEpoch(source) => {
                write!(f, "system clock is before the Unix epoch: {source}")
            }
        }
    }
}

impl Error for ClockError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BeforeUnixEpoch(source) => Some(source),
        }
    }
}

#[derive(Debug)]
pub(crate) enum PublicCacheError {
    Clock(ClockError),
    LockPoisoned(&'static str),
    Redis(data::DataError),
    GenerationUtf8(FromUtf8Error),
    GenerationNumber(std::num::ParseIntError),
    Serialization(serde_json::Error),
}

impl fmt::Display for PublicCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clock(source) => write!(f, "cache clock error: {source}"),
            Self::LockPoisoned(name) => write!(f, "cache {name} lock poisoned"),
            Self::Redis(source) => write!(f, "cache backend error: {source}"),
            Self::GenerationUtf8(source) => {
                write!(f, "cache generation value is not UTF-8: {source}")
            }
            Self::GenerationNumber(source) => {
                write!(f, "cache generation value is not an integer: {source}")
            }
            Self::Serialization(source) => write!(f, "cache serialization error: {source}"),
        }
    }
}

impl Error for PublicCacheError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Clock(source) => Some(source),
            Self::Redis(source) => Some(source),
            Self::GenerationUtf8(source) => Some(source),
            Self::GenerationNumber(source) => Some(source),
            Self::Serialization(source) => Some(source),
            Self::LockPoisoned(_) => None,
        }
    }
}

impl From<ClockError> for PublicCacheError {
    fn from(value: ClockError) -> Self {
        Self::Clock(value)
    }
}
impl From<data::DataError> for PublicCacheError {
    fn from(value: data::DataError) -> Self {
        Self::Redis(value)
    }
}
impl From<serde_json::Error> for PublicCacheError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value)
    }
}

#[derive(Debug)]
pub(crate) enum UploadRuntimeError {
    Storage(storage::FsError),
    Image(storage::ImageError),
}

impl fmt::Display for UploadRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(source) => write!(f, "failed to read stored upload: {source}"),
            Self::Image(source) => write!(f, "invalid uploaded image: {source}"),
        }
    }
}
impl Error for UploadRuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(source) => Some(source),
            Self::Image(source) => Some(source),
        }
    }
}
impl From<storage::FsError> for UploadRuntimeError {
    fn from(value: storage::FsError) -> Self {
        Self::Storage(value)
    }
}
impl From<storage::ImageError> for UploadRuntimeError {
    fn from(value: storage::ImageError) -> Self {
        Self::Image(value)
    }
}
