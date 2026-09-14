use crate::artifact_loader::LoadedArtifact;
use runtime_abi::{RequestValue, VelranHostApi, VelranResult};
use std::collections::BTreeMap;
use std::sync::Arc;

use super::{DispatchError, HandlerBinding};

pub struct ModuleGeneration {
    number: u64,
    shards: BTreeMap<String, Arc<LoadedArtifact>>,
    handlers: BTreeMap<String, HandlerBinding>,
}

impl ModuleGeneration {
    pub fn new(number: u64, shards: BTreeMap<String, Arc<LoadedArtifact>>) -> Self {
        Self {
            number,
            shards,
            handlers: BTreeMap::new(),
        }
    }

    pub fn with_bindings(
        number: u64,
        shards: BTreeMap<String, Arc<LoadedArtifact>>,
        handlers: BTreeMap<String, HandlerBinding>,
    ) -> Self {
        Self {
            number,
            shards,
            handlers,
        }
    }

    pub fn number(&self) -> u64 {
        self.number
    }
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Hot path: one verified handler-name lookup followed by a pre-resolved
    /// native call. `dlopen`, `dlsym`, shard lookup and handler-ID resolution are
    /// activation-time operations only.
    pub fn invoke(
        &self,
        handler_name: &str,
        inputs: &[RequestValue<'_>],
        output: &mut [u8],
        instruction_budget: u64,
        allocation_budget: u64,
    ) -> Result<VelranResult, DispatchError> {
        let binding = self
            .handlers
            .get(handler_name)
            .ok_or_else(|| DispatchError::UnknownShard(handler_name.to_owned()))?;
        binding.invoke(inputs, output, instruction_budget, allocation_budget)
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
        let binding = self
            .handlers
            .get(handler_name)
            .ok_or_else(|| DispatchError::UnknownShard(handler_name.to_owned()))?;
        binding.invoke_with_host(host, inputs, output, instruction_budget, allocation_budget)
    }

    pub fn clone_shards(&self) -> BTreeMap<String, Arc<LoadedArtifact>> {
        self.shards.clone()
    }
    pub fn clone_bindings(&self) -> BTreeMap<String, HandlerBinding> {
        self.handlers.clone()
    }
}
