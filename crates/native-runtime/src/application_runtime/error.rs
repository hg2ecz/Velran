use super::BootstrapError;
use crate::artifact_loader::LoadError;
use crate::runtime_generation::{ActivationError, DispatchError};
use native_build::incremental_build::IncrementalBuildError;
use std::fmt;

#[derive(Debug)]
pub enum RuntimeError {
    Bootstrap(BootstrapError),
    Build(IncrementalBuildError),
    ArtifactLoad(LoadError),
    Activation(ActivationError),
    Dispatch(DispatchError),
    GenerationExhausted,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bootstrap(error) => write!(f, "native runtime bootstrap failed: {error}"),
            Self::Build(error) => write!(f, "incremental build failed: {error}"),
            Self::ArtifactLoad(error) => write!(f, "native artifact load failed: {error}"),
            Self::Activation(error) => write!(f, "generation activation failed: {error}"),
            Self::Dispatch(error) => write!(f, "native dispatch failed: {error}"),
            Self::GenerationExhausted => f.write_str("native generation counter exhausted"),
        }
    }
}
impl std::error::Error for RuntimeError {}

impl RuntimeError {
    pub fn rustc_diagnostics(&self) -> Option<&str> {
        match self {
            Self::Build(error) => error.rustc_diagnostics(),
            _ => None,
        }
    }
}
