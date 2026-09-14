use crate::{FormFailure, PublicError};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    BadRequest,
    FormInvalid(FormFailure),
    UnsupportedMediaType,
    MethodNotAllowed,
    NotFound,
    Forbidden,
    /// Optimistic concurrency conflict (for example a stale version).
    Conflict,
    InstructionLimit,
    MemoryLimit,
    DeadlineExceeded,
    ExternalIoLimit,
    Database,
    Internal,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BadRequest => "bad request",
            Self::FormInvalid(_) => "form validation failed",
            Self::UnsupportedMediaType => "unsupported media type",
            Self::MethodNotAllowed => "method not allowed",
            Self::NotFound => "not found",
            Self::Forbidden => "forbidden",
            Self::Conflict => "conflict",
            Self::InstructionLimit => "instruction limit exceeded",
            Self::MemoryLimit => "runtime allocation limit exceeded",
            Self::DeadlineExceeded => "request execution deadline exceeded",
            Self::ExternalIoLimit => "request external I/O budget exceeded",
            Self::Database => "database operation failed",
            Self::Internal => "internal error",
        })
    }
}

impl std::error::Error for AppError {}

impl From<PublicError> for AppError {
    fn from(value: PublicError) -> Self {
        match value {
            PublicError::BadRequest => Self::BadRequest,
            PublicError::NotFound => Self::NotFound,
            PublicError::Forbidden => Self::Forbidden,
            PublicError::Conflict => Self::Conflict,
        }
    }
}
