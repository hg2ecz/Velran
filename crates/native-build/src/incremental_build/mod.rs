#![forbid(unsafe_code)]

mod error;
mod shard_diff;
mod source_discovery;
mod source_tracking;
use source_discovery::{changed_source_paths, discover_source_paths};
#[cfg(test)]
use source_tracking::hash_source;
use source_tracking::{SourceState, probe_known_sources, snapshot_compiled_sources};

use crate::build_cache::NativeBuildCache;
use crate::rustc_backend::{CompiledArtifact, RustcConfig, compile_shard};
use compiler::{VerifiedCompileError, compile_file_with_dependencies};
use executable_ir::shard_planner::{VerifiedShard, plan};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub use error::IncrementalBuildError;
use shard_diff::{
    changed_fingerprints, dirty_shards, removed_shards, shard_implementation_map,
    shard_interface_map, shard_key_map,
};

#[derive(Debug, Clone)]
struct ApplicationState {
    source_roots: BTreeSet<PathBuf>,
    sources: BTreeMap<PathBuf, SourceState>,
    shard_keys: BTreeMap<String, String>,
    shard_interfaces: BTreeMap<String, String>,
    shard_implementations: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildDisposition {
    InitialBuild,
    NoChanges,
    MetadataOnly,
    SourceChanged,
}

#[derive(Debug)]
pub struct NativeArtifactCandidate {
    pub shard: VerifiedShard,
    pub artifact: CompiledArtifact,
}

pub struct IncrementalBuildReport {
    pub disposition: BuildDisposition,
    pub changed_sources: Vec<PathBuf>,
    pub dirty_shards: Vec<String>,
    pub interface_changed_shards: Vec<String>,
    pub implementation_changed_shards: Vec<String>,
    pub removed_shards: Vec<String>,
    pub native_candidates: Vec<NativeArtifactCandidate>,
}

pub struct PreparedIncrementalBuild {
    report: IncrementalBuildReport,
    next_state: ApplicationState,
}

impl PreparedIncrementalBuild {
    pub fn report(&self) -> &IncrementalBuildReport {
        &self.report
    }
}

pub struct IncrementalBuilder {
    entry: PathBuf,
    rustc: RustcConfig,
    cache: NativeBuildCache,
    state: Option<ApplicationState>,
}

impl IncrementalBuilder {
    pub fn new(
        entry: PathBuf,
        rustc: RustcConfig,
        cache: NativeBuildCache,
    ) -> Result<Self, IncrementalBuildError> {
        if !entry.is_absolute() {
            return Err(IncrementalBuildError::EntryMustBeAbsolute);
        }
        entry
            .parent()
            .ok_or(IncrementalBuildError::EntryMustBeAbsolute)?;
        Ok(Self {
            entry,
            rustc,
            cache,
            state: None,
        })
    }

    pub fn build(&mut self) -> Result<IncrementalBuildReport, IncrementalBuildError> {
        let prepared = self.prepare()?;
        let PreparedIncrementalBuild { report, next_state } = prepared;
        self.state = Some(next_state);
        Ok(report)
    }

    pub fn prepare(&self) -> Result<PreparedIncrementalBuild, IncrementalBuildError> {
        let previous = self.state.clone();
        if let Some(state) = &previous {
            // Detect create/delete/rename before probing known files. Newly dropped
            // `.vrn` modules must trigger compilation even though they were not
            // present in the previous dependency snapshot.
            let discovered = discover_source_paths(&state.source_roots)?;
            let known: std::collections::BTreeSet<PathBuf> =
                state.sources.keys().cloned().collect();
            if discovered != known {
                return self.rebuild(previous);
            }
            let probe = probe_known_sources(&state.sources)?;
            if probe.metadata_changed.is_empty() {
                return Ok(PreparedIncrementalBuild {
                    report: empty_report(BuildDisposition::NoChanges),
                    next_state: state.clone(),
                });
            }
            if probe.content_changed.is_empty() {
                let mut refreshed = state.clone();
                for (path, source) in probe.refreshed {
                    refreshed.sources.insert(path, source);
                }
                return Ok(PreparedIncrementalBuild {
                    report: IncrementalBuildReport {
                        disposition: BuildDisposition::MetadataOnly,
                        changed_sources: probe.metadata_changed,
                        dirty_shards: Vec::new(),
                        interface_changed_shards: Vec::new(),
                        implementation_changed_shards: Vec::new(),
                        removed_shards: Vec::new(),
                        native_candidates: Vec::new(),
                    },
                    next_state: refreshed,
                });
            }
        }
        self.rebuild(previous)
    }

    pub fn commit(&mut self, prepared: PreparedIncrementalBuild) -> IncrementalBuildReport {
        let PreparedIncrementalBuild { report, next_state } = prepared;
        self.state = Some(next_state);
        report
    }

    fn rebuild(
        &self,
        previous: Option<ApplicationState>,
    ) -> Result<PreparedIncrementalBuild, IncrementalBuildError> {
        const MAX_STABLE_SOURCE_ATTEMPTS: usize = 4;
        let mut last_changed = None;
        for _ in 0..MAX_STABLE_SOURCE_ATTEMPTS {
            match self.rebuild_once(previous.clone()) {
                Err(IncrementalBuildError::SourceChangedDuringHash(path)) => {
                    last_changed = Some(path);
                    continue;
                }
                result => return result,
            }
        }
        Err(IncrementalBuildError::SourceChangedDuringHash(
            last_changed.unwrap_or_else(|| self.entry.clone()),
        ))
    }

    fn rebuild_once(
        &self,
        previous: Option<ApplicationState>,
    ) -> Result<PreparedIncrementalBuild, IncrementalBuildError> {
        let compiled = compile_file_with_dependencies(&self.entry).map_err(|error| {
            IncrementalBuildError::Compile(VerifiedCompileError::Compile(error))
        })?;
        let verified = executable_ir::verify(&compiled.program)
            .map_err(|error| IncrementalBuildError::Compile(VerifiedCompileError::Verify(error)))?;
        let shards = plan(&verified);
        let sources = snapshot_compiled_sources(&compiled.sources)?;
        let shard_keys = shard_key_map(&shards, &self.cache, &self.rustc);
        let shard_interfaces = shard_interface_map(&shards);
        let shard_implementations = shard_implementation_map(&shards);

        let previous_keys = previous.as_ref().map(|state| &state.shard_keys);
        let dirty = dirty_shards(&shards, previous_keys, &shard_keys);
        let removed = removed_shards(previous_keys, &shard_keys);
        let interface_changed = changed_fingerprints(
            previous.as_ref().map(|state| &state.shard_interfaces),
            &shard_interfaces,
        );
        let implementation_changed = changed_fingerprints(
            previous.as_ref().map(|state| &state.shard_implementations),
            &shard_implementations,
        );
        let changed_sources =
            changed_source_paths(previous.as_ref().map(|state| &state.sources), &sources);
        let disposition = if previous.is_none() {
            BuildDisposition::InitialBuild
        } else {
            BuildDisposition::SourceChanged
        };

        let mut native_candidates = Vec::with_capacity(dirty.len());
        for shard in &dirty {
            if !is_native_eligible(shard) {
                return Err(IncrementalBuildError::UnsupportedNativeShard(
                    shard.id().as_str().to_owned(),
                ));
            }
            let artifact = compile_shard(shard, &self.rustc, &self.cache)
                .map_err(IncrementalBuildError::Native)?;
            native_candidates.push(NativeArtifactCandidate {
                shard: (*shard).clone(),
                artifact,
            });
        }
        let dirty_names = dirty
            .iter()
            .map(|shard| shard.id().as_str().to_owned())
            .collect();
        Ok(PreparedIncrementalBuild {
            report: IncrementalBuildReport {
                disposition,
                changed_sources,
                dirty_shards: dirty_names,
                interface_changed_shards: interface_changed,
                implementation_changed_shards: implementation_changed,
                removed_shards: removed,
                native_candidates,
            },
            next_state: ApplicationState {
                source_roots: compiled.source_roots.iter().cloned().collect(),
                sources,
                shard_keys,
                shard_interfaces,
                shard_implementations,
            },
        })
    }
}

fn is_native_eligible(shard: &VerifiedShard) -> bool {
    shard.native_eligible()
}

fn empty_report(disposition: BuildDisposition) -> IncrementalBuildReport {
    IncrementalBuildReport {
        disposition,
        changed_sources: Vec::new(),
        dirty_shards: Vec::new(),
        interface_changed_shards: Vec::new(),
        implementation_changed_shards: Vec::new(),
        removed_shards: Vec::new(),
        native_candidates: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
