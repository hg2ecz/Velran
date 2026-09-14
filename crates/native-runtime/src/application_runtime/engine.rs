use crate::artifact_loader::LoadedArtifact;
use crate::runtime_generation::{ActiveGeneration, HandlerBinding, ModuleGeneration};
use native_build::incremental_build::{
    BuildDisposition, IncrementalBuilder, PreparedIncrementalBuild,
};
use observability::server_event;
use runtime_abi::{RequestValue, VelranHostApi, VelranResult};
use std::collections::BTreeMap;
use std::sync::Arc;

use super::{RefreshReport, RuntimeError};

pub struct ApplicationRuntime {
    builder: IncrementalBuilder,
    active: ActiveGeneration,
    next_generation: u64,
}

impl ApplicationRuntime {
    pub fn new(builder: IncrementalBuilder) -> Self {
        Self {
            builder,
            active: ActiveGeneration::new(ModuleGeneration::new(0, Default::default())),
            next_generation: 1,
        }
    }

    pub fn refresh(&mut self) -> Result<RefreshReport, RuntimeError> {
        let prepared = self.builder.prepare().map_err(RuntimeError::Build)?;
        if matches!(
            prepared.report().disposition,
            BuildDisposition::NoChanges | BuildDisposition::MetadataOnly
        ) {
            let generation = self
                .active
                .pin()
                .map_err(RuntimeError::Activation)?
                .number();
            let report = self.builder.commit(prepared);
            return Ok(RefreshReport {
                build_disposition: report.disposition,
                generation,
                activated: false,
                dirty_shards: report.dirty_shards,
            });
        }
        self.activate_prepared(prepared)
    }

    pub fn pin(&self) -> Result<crate::runtime_generation::GenerationPin, RuntimeError> {
        self.active.pin().map_err(RuntimeError::Activation)
    }

    pub fn native_shard_count(&self) -> Result<usize, RuntimeError> {
        Ok(self.pin()?.shard_count())
    }
    pub fn generation(&self) -> Result<u64, RuntimeError> {
        Ok(self.pin()?.number())
    }

    pub fn dispatch(
        &self,
        handler_name: &str,
        inputs: &[RequestValue<'_>],
        output: &mut [u8],
        instruction_budget: u64,
        allocation_budget: u64,
    ) -> Result<VelranResult, RuntimeError> {
        self.pin()?
            .invoke(
                handler_name,
                inputs,
                output,
                instruction_budget,
                allocation_budget,
            )
            .map_err(RuntimeError::Dispatch)
    }
    pub fn dispatch_with_host(
        &self,
        handler_name: &str,
        host: &VelranHostApi,
        inputs: &[RequestValue<'_>],
        output: &mut [u8],
        instruction_budget: u64,
        allocation_budget: u64,
    ) -> Result<VelranResult, RuntimeError> {
        self.pin()?
            .invoke_with_host(
                handler_name,
                host,
                inputs,
                output,
                instruction_budget,
                allocation_budget,
            )
            .map_err(RuntimeError::Dispatch)
    }

    pub fn reap_retired(&self) -> Result<usize, RuntimeError> {
        self.active.reap_retired().map_err(RuntimeError::Activation)
    }

    fn activate_prepared(
        &mut self,
        prepared: PreparedIncrementalBuild,
    ) -> Result<RefreshReport, RuntimeError> {
        let current = self.active.pin().map_err(RuntimeError::Activation)?;
        let mut shards = current.clone_shards();
        let mut bindings = current.clone_bindings();

        for shard_id in &prepared.report().removed_shards {
            shards.remove(shard_id);
            bindings.retain(|_, binding| binding.shard_id() != shard_id);
        }

        self.load_candidates(&prepared, &mut shards, &mut bindings)?;

        let generation = self.next_generation;
        let following_generation = generation
            .checked_add(1)
            .ok_or(RuntimeError::GenerationExhausted)?;
        let previous = self
            .active
            .activate(ModuleGeneration::with_bindings(
                generation, shards, bindings,
            ))
            .map_err(RuntimeError::Activation)?;
        let report = self.builder.commit(prepared);
        self.next_generation = following_generation;
        server_event(
            "info",
            "native_generation_activated",
            "application-runtime",
            &format!(
                "previous_generation={previous} generation={generation} dirty_shards={}",
                report.dirty_shards.len()
            ),
        );
        Ok(RefreshReport {
            build_disposition: report.disposition,
            generation,
            activated: true,
            dirty_shards: report.dirty_shards,
        })
    }

    fn load_candidates(
        &self,
        prepared: &PreparedIncrementalBuild,
        shards: &mut BTreeMap<String, Arc<LoadedArtifact>>,
        bindings: &mut BTreeMap<String, HandlerBinding>,
    ) -> Result<(), RuntimeError> {
        for candidate in &prepared.report().native_candidates {
            server_event(
                "info",
                if candidate.artifact.cache_hit {
                    "native_binary_cache_hit"
                } else {
                    "native_binary_cache_miss_compiled"
                },
                "application-runtime",
                &format!(
                    "shard={} artifact={}",
                    candidate.shard.id().as_str(),
                    candidate.artifact.path.display()
                ),
            );
            let artifact = Arc::new(
                LoadedArtifact::load(
                    &candidate.shard,
                    &candidate.artifact.path,
                    &candidate.artifact.manifest,
                )
                .map_err(RuntimeError::ArtifactLoad)?,
            );
            let shard_id = candidate.shard.id().as_str().to_owned();
            shards.insert(shard_id.clone(), Arc::clone(&artifact));
            bindings.retain(|_, binding| binding.shard_id() != shard_id);
            for (handler_id, handler_name) in candidate.shard.handlers().keys().enumerate() {
                let handler_id =
                    u32::try_from(handler_id).map_err(|_| RuntimeError::GenerationExhausted)?;
                bindings.insert(
                    handler_name.clone(),
                    HandlerBinding::new(shard_id.clone(), handler_id, Arc::clone(&artifact)),
                );
            }
        }
        Ok(())
    }
}
