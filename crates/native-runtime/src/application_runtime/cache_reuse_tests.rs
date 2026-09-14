use super::{NativeOptimization, NativeRuntimeConfig, from_config};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn enabled() -> bool {
    std::env::var_os("VELRAN_RELEASE_CACHE_SMOKE").as_deref() == Some(std::ffi::OsStr::new("1"))
}

fn temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("velran-cache-reuse-{}-{nonce}", std::process::id()));
    fs::create_dir(&path).unwrap();
    path
}

fn real_rustc() -> Option<PathBuf> {
    let launcher = std::env::var_os("RUSTC")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() || path.components().count() > 1)
        .or_else(|| {
            let path = std::env::var_os("PATH")?;
            std::env::split_paths(&path)
                .map(|dir| dir.join(format!("rustc{}", std::env::consts::EXE_SUFFIX)))
                .find(|candidate| candidate.is_file())
        })?;
    let output = std::process::Command::new(launcher)
        .args(["--print", "sysroot"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sysroot = String::from_utf8(output.stdout).ok()?;
    let rustc = PathBuf::from(sysroot.trim())
        .join("bin")
        .join(format!("rustc{}", std::env::consts::EXE_SUFFIX));
    rustc.is_file().then_some(rustc)
}

#[cfg(unix)]
fn write_counting_rustc(wrapper: &Path, rustc: &Path, log: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec '{}' \"$@\"\n",
        log.display(),
        rustc.display(),
    );
    fs::write(wrapper, script).unwrap();
    let mut permissions = fs::metadata(wrapper).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(wrapper, permissions).unwrap();
}

#[cfg(unix)]
#[test]
fn second_bootstrap_uses_binary_cache_without_any_rustc_process() {
    if !enabled() {
        eprintln!("SKIP cache reuse smoke: set VELRAN_RELEASE_CACHE_SMOKE=1");
        return;
    }
    let rustc = real_rustc().expect("release cache smoke requires rustc");
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("native-runtime must be inside workspace/crates")
        .to_path_buf();
    let entry = workspace.join("examples/typed-data-fastpaths/main.vrn");
    let dir = temp_dir();
    let wrapper = dir.join("rustc-counting-wrapper");
    let log = dir.join("rustc-invocations.log");
    write_counting_rustc(&wrapper, &rustc, &log);
    let config = NativeRuntimeConfig {
        enabled: true,
        rustc_path: Some(wrapper),
        cache_root: Some(dir.join("cache")),
        optimization: NativeOptimization::Release,
        target_cpu: None,
        cpu_features: None,
        required: true,
        debug_rustc_repro: false,
    };

    let first =
        from_config(&entry, &config).expect("first bootstrap must compile and load native shard");
    assert!(first.native_shard_count().unwrap() > 0);
    drop(first);
    let after_first = fs::read_to_string(&log).expect("first bootstrap must invoke rustc");
    let first_invocations = after_first.lines().count();
    assert!(
        first_invocations >= 2,
        "first bootstrap must probe rustc and compile at least one shard"
    );

    let second = from_config(&entry, &config).expect("second bootstrap must load native cache");
    assert!(second.native_shard_count().unwrap() > 0);
    drop(second);
    let after_second = fs::read_to_string(&log).expect("rustc invocation log must remain present");
    assert_eq!(
        after_second.lines().count(),
        first_invocations,
        "warm native cache bootstrap must not launch rustc at all",
    );
    fs::remove_dir_all(dir).unwrap();
}
