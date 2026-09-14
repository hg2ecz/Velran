use std::fs;
use std::path::{Path, PathBuf};

fn enabled() -> bool {
    std::env::var_os("VELRAN_RELEASE_NATIVE_SMOKE").as_deref() == Some(std::ffi::OsStr::new("1"))
}

#[test]
fn release_native_positive_smoke_compiles_generated_cdylibs() {
    if !enabled() {
        eprintln!("SKIP release native smoke: set VELRAN_RELEASE_NATIVE_SMOKE=1");
        return;
    }
    let config = crate::rustc_backend::test_toolchain_config(
        crate::rustc_backend::OptimizationProfile::Release,
    )
    .expect("VELRAN release native smoke requires a resolvable rustc toolchain");
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("native-build must be inside workspace/crates")
        .to_path_buf();
    let entries = [
        "examples/fft4096-x10000/main.vrn",
        "examples/typed-data-fastpaths/main.vrn",
        "examples/pure-fn-tagged-sum/main.vrn",
        "examples/pure-fn-typed-struct/main.vrn",
        "examples/typed-query-form/main.vrn",
        "examples/typed-multipart/main.vrn",
        "examples/json-typed-response/app.vrn",
    ];
    let dir = super::temp_dir();
    let cache = crate::build_cache::NativeBuildCache::open(dir.join("cache")).unwrap();
    for relative in entries {
        let entry = workspace.join(relative);
        let compiled = compiler::compile_file_with_dependencies(&entry)
            .unwrap_or_else(|error| panic!("frontend compile failed for {relative}: {error:?}"));
        let verified = executable_ir::verify(&compiled.program)
            .unwrap_or_else(|error| panic!("EIR verification failed for {relative}: {error:?}"));
        let shards = executable_ir::shard_planner::plan(&verified);
        assert!(
            !shards.is_empty(),
            "release native smoke produced no shard for {relative}"
        );
        for shard in &shards {
            assert!(
                shard.native_eligible(),
                "release native smoke found non-native shard {} in {relative}",
                shard.id().as_str()
            );
            let artifact = crate::rustc_backend::compile_shard(shard, &config, &cache)
                .unwrap_or_else(|error| {
                    panic!(
                        "direct rustc shard compile failed for {relative}/{}: {error}",
                        shard.id().as_str()
                    )
                });
            assert!(
                artifact.path.is_file(),
                "compiled artifact missing for {relative}"
            );
        }
    }
    fs::remove_dir_all(dir).unwrap();
}
