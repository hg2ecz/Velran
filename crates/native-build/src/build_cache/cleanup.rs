use super::{CacheError, io_error};
use std::fs;
use std::path::Path;

#[cfg(target_os = "linux")]
pub(super) fn abandoned_staging(root: &Path) -> Result<(), CacheError> {
    for entry in fs::read_dir(root).map_err(|e| io_error("scan cache staging", e))? {
        let entry = entry.map_err(|e| io_error("read cache staging entry", e))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(pid) = staging_owner_pid(name) else {
            continue;
        };
        if Path::new("/proc").join(pid.to_string()).exists() {
            continue;
        }
        let path = entry.path();
        let metadata =
            fs::symlink_metadata(&path).map_err(|e| io_error("stat abandoned cache staging", e))?;
        if metadata.file_type().is_symlink() || metadata.is_file() {
            fs::remove_file(&path)
                .map_err(|e| io_error("remove abandoned cache staging file", e))?;
        } else if metadata.is_dir() {
            fs::remove_dir_all(&path)
                .map_err(|e| io_error("remove abandoned cache staging directory", e))?;
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub(super) fn abandoned_staging(_: &Path) -> Result<(), CacheError> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn staging_owner_pid(name: &str) -> Option<u32> {
    if let Some(rest) = name
        .strip_prefix(".rustc-identity-v1.")
        .and_then(|v| v.strip_suffix(".tmp"))
    {
        return rest.parse().ok();
    }
    if let Some(rest) = name.strip_prefix(".work-") {
        let mut parts = rest.rsplitn(3, '-');
        let nonce = parts.next()?;
        let pid = parts.next()?;
        let key = parts.next()?;
        if key.len() == 16
            && key.bytes().all(|b| b.is_ascii_hexdigit())
            && nonce.bytes().all(|b| b.is_ascii_digit())
        {
            return pid.parse().ok();
        }
    }
    if let Some(rest) = name.strip_prefix('.').and_then(|v| v.strip_suffix(".tmp")) {
        let mut parts = rest.split('.');
        let key = parts.next()?;
        let pid = parts.next()?;
        let nonce = parts.next()?;
        if parts.next().is_none()
            && key.len() == 64
            && key.bytes().all(|b| b.is_ascii_hexdigit())
            && nonce.bytes().all(|b| b.is_ascii_digit())
        {
            return pid.parse().ok();
        }
    }
    None
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn removes_only_abandoned_known_staging_names() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "velran-cache-cleanup-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let dead_pid = u32::MAX;
        let cache_tmp = root.join(format!(".{}.{}.1.tmp", "a".repeat(64), dead_pid));
        let work_tmp = root.join(format!(".work-{}-{}-1", "b".repeat(16), dead_pid));
        let identity_tmp = root.join(format!(".rustc-identity-v1.{dead_pid}.tmp"));
        let unrelated = root.join("keep-me");
        fs::create_dir(&cache_tmp).unwrap();
        fs::create_dir(&work_tmp).unwrap();
        fs::write(&identity_tmp, b"partial").unwrap();
        fs::write(&unrelated, b"safe").unwrap();
        abandoned_staging(&root).unwrap();
        assert!(!cache_tmp.exists());
        assert!(!work_tmp.exists());
        assert!(!identity_tmp.exists());
        assert!(unrelated.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
