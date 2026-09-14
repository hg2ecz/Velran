#![forbid(unsafe_code)]

use std::path::PathBuf;

use executable_ir::shard_planner::VerifiedShard;

use crate::artifact_format::ArtifactManifest;
use crate::build_cache::NativeBuildCache;

use super::{BuildError, RustcConfig, build, io_error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledArtifact {
    pub crate_name: String,
    pub path: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: ArtifactManifest,
    pub cache_hit: bool,
}

pub fn compile_shard(
    shard: &VerifiedShard,
    config: &RustcConfig,
    cache: &NativeBuildCache,
) -> Result<CompiledArtifact, BuildError> {
    let key = cache.key_for(shard, &config.cache_identity());
    if !config.repro_diagnostics() {
        if let Some(hit) = cache.lookup(&key, shard).map_err(BuildError::Cache)? {
            return Ok(CompiledArtifact {
                crate_name: hit.manifest.crate_name.clone(),
                path: hit.artifact_path,
                manifest_path: hit.manifest_path,
                manifest: hit.manifest,
                cache_hit: true,
            });
        }
    } else {
        eprintln!("VELRAN_RUSTC_REPRO_CACHE_BYPASS=1");
    }

    let work_dir = cache.root().join(format!(
        ".work-{}-{}-{}",
        &key.as_str()[..16],
        std::process::id(),
        next_work_nonce()
    ));
    if work_dir.exists() {
        std::fs::remove_dir_all(&work_dir)
            .map_err(|error| io_error("remove stale rustc work directory", error))?;
    }
    std::fs::create_dir(&work_dir)
        .map_err(|error| io_error("create rustc work directory", error))?;
    let built = build::compile(shard, config, &work_dir)?;
    let hit = cache
        .store(&key, shard, &built.path, &built.manifest)
        .map_err(BuildError::Cache)?;
    if !config.repro_diagnostics() {
        std::fs::remove_dir_all(&work_dir)
            .map_err(|error| io_error("remove rustc work directory", error))?;
    } else {
        eprintln!("VELRAN_RUSTC_REPRO_DIR={}", work_dir.display());
    }
    Ok(CompiledArtifact {
        crate_name: hit.manifest.crate_name.clone(),
        path: hit.artifact_path,
        manifest_path: hit.manifest_path,
        manifest: hit.manifest,
        cache_hit: false,
    })
}

fn next_work_nonce() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
