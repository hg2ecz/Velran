use crate::artifact_loader::LoadError;
use std::fmt;

#[derive(Debug)]
pub enum DispatchError {
    UnknownShard(String),
    NativeInvoke(LoadError),
}
impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownShard(id) => write!(f, "unknown native shard `{id}`"),
            Self::NativeInvoke(error) => write!(f, "native invocation failed: {error}"),
        }
    }
}
impl std::error::Error for DispatchError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationError {
    LockPoisoned,
    GenerationNotMonotonic { current: u64, next: u64 },
}
impl fmt::Display for ActivationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LockPoisoned => f.write_str("active generation lock poisoned"),
            Self::GenerationNotMonotonic { current, next } => write!(
                f,
                "generation must increase monotonically: current {current}, next {next}"
            ),
        }
    }
}
impl std::error::Error for ActivationError {}
