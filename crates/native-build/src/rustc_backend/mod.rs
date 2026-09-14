#![forbid(unsafe_code)]

mod abi_host;
mod abi_input_validation;
mod abi_shim;
#[cfg(test)]
mod abi_shim_tests;
mod build;
mod command;
mod compile;
mod config;
mod diagnostics;
mod error;
mod publish;

pub use compile::{CompiledArtifact, compile_shard};
#[cfg(test)]
pub(crate) use config::test_toolchain_config;
pub use config::{OptimizationProfile, RustcConfig, ToolchainIdentity};
pub use error::BuildError;
pub(crate) use error::io_error;
