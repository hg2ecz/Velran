use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "velran-incremental-test-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&path).unwrap();
    path
}

#[test]
fn source_hash_detects_content_change() {
    let dir = temp_dir();
    let path = dir.join("main.vrn");
    fs::write(&path, "#[page] fn a(ctx: PageContext) -> Result<Json, PageError> { return Ok(json(1)); }\nroute a GET \"/\" public => a;").unwrap();
    let first = hash_source(&path).unwrap();
    fs::write(&path, "#[page] fn a(ctx: PageContext) -> Result<Json, PageError> { return Ok(json(2)); }\nroute a GET \"/\" public => a;").unwrap();
    let second = hash_source(&path).unwrap();
    assert_ne!(first.sha256, second.sha256);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_tracked_source_forces_content_rebuild() {
    let dir = temp_dir();
    let path = dir.join("module.vrn");
    fs::write(&path, "#[page] fn a(ctx: PageContext) -> Result<Json, PageError> { return Ok(json(1)); }\nroute a GET \"/\" public => a;").unwrap();
    let state = hash_source(&path).unwrap();
    fs::remove_file(&path).unwrap();
    let probe = probe_known_sources(&BTreeMap::from([(path.clone(), state)])).unwrap();
    assert_eq!(probe.content_changed, vec![path]);
    fs::remove_dir_all(dir).unwrap();
}

fn test_rustc_config() -> crate::rustc_backend::RustcConfig {
    crate::rustc_backend::test_toolchain_config(
        crate::rustc_backend::OptimizationProfile::Interactive,
    )
    .expect("cargo test requires a resolvable rustc toolchain")
}

#[test]
fn prepare_is_transactional_until_commit() {
    let dir = temp_dir();
    let cache_dir = dir.join("cache");
    let app = dir.join("main.vrn");
    fs::write(
        &app,
        "#[page] fn a(ctx: PageContext) -> Result<Json, PageError> { return Ok(json(1)); }\nroute a GET \"/\" public => a;",
    )
    .unwrap();
    let cache = crate::build_cache::NativeBuildCache::open(cache_dir).unwrap();
    let mut builder = IncrementalBuilder::new(app, test_rustc_config(), cache).unwrap();

    let first = builder.prepare().unwrap();
    assert_eq!(first.report().disposition, BuildDisposition::InitialBuild);
    let second = builder.prepare().unwrap();
    assert_eq!(second.report().disposition, BuildDisposition::InitialBuild);

    let committed = builder.commit(first);
    assert_eq!(committed.disposition, BuildDisposition::InitialBuild);
    let after_commit = builder.prepare().unwrap();
    assert_eq!(
        after_commit.report().disposition,
        BuildDisposition::NoChanges
    );
    fs::remove_dir_all(dir).unwrap();
}

mod release_native_smoke;
