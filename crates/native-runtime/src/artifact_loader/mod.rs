#![deny(unsafe_op_in_unsafe_fn)]
use executable_ir::shard_planner::VerifiedShard;
use native_build::artifact_format::ArtifactManifest;
use runtime_abi::{
    MAX_INPUT_FIELDS, MAX_OUTPUT_BYTES, RequestValue, VelranHostApi, VelranInputValue, VelranResult,
};
use std::path::{Path, PathBuf};
#[cfg(unix)]
mod contract;
mod error;
mod input;
mod path_validation;
#[cfg(unix)]
mod unix;
#[cfg(unix)]
mod unix_abi;
#[cfg(unix)]
mod unix_symbols;
pub use error::LoadError;
pub struct LoadedArtifact {
    path: PathBuf,
    #[cfg(unix)]
    library: unix::Library,
}
impl LoadedArtifact {
    pub fn load(
        shard: &VerifiedShard,
        path: &Path,
        manifest: &ArtifactManifest,
    ) -> Result<Self, LoadError> {
        path_validation::validate(path)?;
        manifest
            .verify_for(shard, path)
            .map_err(LoadError::Integrity)?;
        #[cfg(unix)]
        {
            let library = unix::Library::open(path)?;
            contract::verify(&library, shard)?;
            return Ok(Self {
                path: path.to_path_buf(),
                library,
            });
        }
        #[cfg(not(unix))]
        {
            let _ = (shard, manifest);
            Err(LoadError::UnsupportedPlatform)
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn invoke(
        &self,
        handler_id: u32,
        host: &VelranHostApi,
        inputs: &[RequestValue<'_>],
        output: &mut [u8],
        instruction_budget: u64,
        allocation_budget: u64,
    ) -> Result<VelranResult, LoadError> {
        if inputs.len() > MAX_INPUT_FIELDS || output.len() > MAX_OUTPUT_BYTES {
            return Err(LoadError::MalformedInput);
        }
        let mut wire = [VelranInputValue::EMPTY; MAX_INPUT_FIELDS];
        for (slot, value) in wire.iter_mut().zip(inputs) {
            *slot = input::wire(value);
        }
        let wire = &wire[..inputs.len()];
        #[cfg(unix)]
        {
            let result = self.library.invoke(
                handler_id,
                host,
                &wire,
                output,
                instruction_budget,
                allocation_budget,
            );
            if !result.is_well_formed() {
                return Err(LoadError::MalformedResult);
            }
            Ok(result)
        }
        #[cfg(not(unix))]
        {
            let _ = (
                handler_id,
                wire,
                output,
                instruction_budget,
                allocation_budget,
            );
            Err(LoadError::UnsupportedPlatform)
        }
    }
}
