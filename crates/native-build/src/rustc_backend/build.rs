#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use compiler::codegen::generate;
use executable_ir::shard_planner::VerifiedShard;

use crate::artifact_format::ArtifactManifest;

use super::{
    BuildError, CompiledArtifact, RustcConfig, abi_shim, command, diagnostics, io_error, publish,
};

pub(crate) fn compile(
    shard: &VerifiedShard,
    config: &RustcConfig,
    build_dir: &Path,
) -> Result<CompiledArtifact, BuildError> {
    let generated = generate(shard).map_err(BuildError::UnsupportedLowering)?;
    fs::create_dir_all(build_dir).map_err(|error| io_error("create build directory", error))?;

    let source_path = build_dir.join(format!("{}.rs", generated.crate_name));
    let staged_cdylib = build_dir.join(publish::dynamic_library_name(&format!(
        "{}.staged",
        generated.crate_name
    )));
    let mut source = String::new();
    source.push_str("#![deny(warnings)]\n#![deny(unsafe_op_in_unsafe_fn)]\n\n");
    source.push_str(&generated.source);
    source.push('\n');
    source.push_str(abi_shim::source());
    fs::write(&source_path, &source)
        .map_err(|error| io_error("write generated cdylib Rust", error))?;
    if config.repro_diagnostics() {
        diagnostics::print_generated_source(&source_path, &source);
    }
    command::run_rustc(
        config,
        command::cdylib_args(&generated.crate_name, &source_path, &staged_cdylib, config),
        "direct shard cdylib compilation",
    )?;

    let staged =
        ArtifactManifest::from_artifact(shard, generated.crate_name.clone(), &staged_cdylib)
            .map_err(BuildError::Integrity)?;
    let hash_prefix = &staged.artifact_sha256[..16];
    let final_path = build_dir.join(publish::dynamic_library_name(&format!(
        "{}-{hash_prefix}",
        generated.crate_name
    )));
    let manifest_path = build_dir.join(format!("{}-{hash_prefix}.manifest", generated.crate_name));
    publish::publish_immutable(
        &staged_cdylib,
        &final_path,
        shard,
        &generated.crate_name,
        &staged,
    )?;

    let manifest =
        ArtifactManifest::from_artifact(shard, generated.crate_name.clone(), &final_path)
            .map_err(BuildError::Integrity)?;
    manifest
        .verify_for(shard, &final_path)
        .map_err(BuildError::Integrity)?;
    publish::atomic_write(&manifest_path, manifest.encode().as_bytes())?;
    Ok(CompiledArtifact {
        crate_name: generated.crate_name,
        path: final_path,
        manifest_path,
        manifest,
        cache_hit: false,
    })
}
