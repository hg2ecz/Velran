use crate::artifact_loader::LoadedArtifact;
use runtime_abi::{RequestValue, VelranHostApi, VelranResult};
use std::sync::Arc;

use super::DispatchError;

/// Fully resolved handler binding. The shard library is pinned by `Arc` and the
/// handler ID is fixed at generation activation, so request dispatch performs a
/// single handler-table lookup and no shard lookup or dynamic-symbol lookup.
#[derive(Clone)]
pub struct HandlerBinding {
    shard_id: String,
    handler_id: u32,
    artifact: Arc<LoadedArtifact>,
}

impl HandlerBinding {
    pub fn new(shard_id: String, handler_id: u32, artifact: Arc<LoadedArtifact>) -> Self {
        Self {
            shard_id,
            handler_id,
            artifact,
        }
    }
    pub fn shard_id(&self) -> &str {
        &self.shard_id
    }
    pub fn handler_id(&self) -> u32 {
        self.handler_id
    }

    pub(crate) fn invoke(
        &self,
        inputs: &[RequestValue<'_>],
        output: &mut [u8],
        instruction_budget: u64,
        allocation_budget: u64,
    ) -> Result<VelranResult, DispatchError> {
        self.invoke_with_host(
            &VelranHostApi::UNAVAILABLE,
            inputs,
            output,
            instruction_budget,
            allocation_budget,
        )
    }

    pub(crate) fn invoke_with_host(
        &self,
        host: &VelranHostApi,
        inputs: &[RequestValue<'_>],
        output: &mut [u8],
        instruction_budget: u64,
        allocation_budget: u64,
    ) -> Result<VelranResult, DispatchError> {
        self.artifact
            .invoke(
                self.handler_id,
                host,
                inputs,
                output,
                instruction_budget,
                allocation_budget,
            )
            .map_err(DispatchError::NativeInvoke)
    }
}
