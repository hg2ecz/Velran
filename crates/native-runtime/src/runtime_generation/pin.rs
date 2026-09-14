use std::collections::BTreeMap;
use std::sync::Arc;

use crate::artifact_loader::LoadedArtifact;
use runtime_abi::{RequestValue, VelranHostApi, VelranResult};

use super::{DispatchError, HandlerBinding, ModuleGeneration};

pub struct GenerationPin(pub(crate) Arc<ModuleGeneration>);

impl GenerationPin {
    pub fn number(&self) -> u64 {
        self.0.number()
    }
    pub fn shard_count(&self) -> usize {
        self.0.shard_count()
    }
    pub fn handler_count(&self) -> usize {
        self.0.handler_count()
    }
    pub fn invoke(
        &self,
        handler_name: &str,
        inputs: &[RequestValue<'_>],
        output: &mut [u8],
        instruction_budget: u64,
        allocation_budget: u64,
    ) -> Result<VelranResult, DispatchError> {
        self.0.invoke(
            handler_name,
            inputs,
            output,
            instruction_budget,
            allocation_budget,
        )
    }
    pub fn invoke_with_host(
        &self,
        handler_name: &str,
        host: &VelranHostApi,
        inputs: &[RequestValue<'_>],
        output: &mut [u8],
        instruction_budget: u64,
        allocation_budget: u64,
    ) -> Result<VelranResult, DispatchError> {
        self.0.invoke_with_host(
            handler_name,
            host,
            inputs,
            output,
            instruction_budget,
            allocation_budget,
        )
    }
    pub fn clone_shards(&self) -> BTreeMap<String, Arc<LoadedArtifact>> {
        self.0.clone_shards()
    }
    pub fn clone_bindings(&self) -> BTreeMap<String, HandlerBinding> {
        self.0.clone_bindings()
    }
}
