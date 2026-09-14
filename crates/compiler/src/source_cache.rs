use crate::artifact_contract;
use crate::source_syntax::is_identifier;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const CACHE_FORMAT: &str = "velran-source-mods-v1";
const MAX_CACHE_ENTRY_BYTES: u64 = 64 * 1024;

static HITS: AtomicU64 = AtomicU64::new(0);
static MISSES: AtomicU64 = AtomicU64::new(0);
static REJECTS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompilerCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub rejects: u64,
}

pub fn stats() -> CompilerCacheStats {
    CompilerCacheStats {
        hits: HITS.load(Ordering::Relaxed),
        misses: MISSES.load(Ordering::Relaxed),
        rejects: REJECTS.load(Ordering::Relaxed),
    }
}

pub(crate) fn load_mods(source: &[u8]) -> Option<Vec<Vec<String>>> {
    let key = cache_key(source);
    let path = entry_path(&key)?;
    let metadata = fs::symlink_metadata(&path).ok()?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_CACHE_ENTRY_BYTES
    {
        REJECTS.fetch_add(1, Ordering::Relaxed);
        return None;
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let file = OpenOptions::new().read(true).open(&path).ok()?;
    let mut limited = file.take(MAX_CACHE_ENTRY_BYTES + 1);
    limited.read_to_end(&mut bytes).ok()?;
    match decode(&bytes, &key) {
        Some(mods) => {
            HITS.fetch_add(1, Ordering::Relaxed);
            Some(mods)
        }
        None => {
            REJECTS.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}

pub(crate) fn store_mods(source: &[u8], mods: &[Vec<String>]) {
    MISSES.fetch_add(1, Ordering::Relaxed);
    let key = cache_key(source);
    let Some(path) = entry_path(&key) else { return };
    let Some(parent) = path.parent() else { return };
    if ensure_cache_dir(parent).is_none() {
        return;
    }
    let payload = encode(&key, mods);
    if payload.len() as u64 > MAX_CACHE_ENTRY_BYTES {
        return;
    }
    let temp = parent.join(format!(".{key}.{}.tmp", std::process::id()));
    let write_result = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| {
            file.write_all(&payload)?;
            file.sync_all()
        });
    if write_result.is_ok() {
        let _ = fs::rename(&temp, &path);
    }
    let _ = fs::remove_file(temp);
}

fn entry_path(key: &str) -> Option<PathBuf> {
    let dir = std::env::var_os("VELRAN_COMPILER_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("velran-compiler-cache-v1"));
    ensure_cache_dir(&dir)?;
    Some(dir.join(format!("{key}.mods")))
}

fn ensure_cache_dir(path: &Path) -> Option<()> {
    fs::create_dir_all(path).ok()?;
    let meta = fs::symlink_metadata(path).ok()?;
    (!meta.file_type().is_symlink() && meta.is_dir()).then_some(())
}

fn cache_key(source: &[u8]) -> String {
    artifact_contract::content_key(CACHE_FORMAT, source)
}

fn encode(key: &str, mods: &[Vec<String>]) -> Vec<u8> {
    let mut out = format!("{CACHE_FORMAT}\n{key}\n").into_bytes();
    for module in mods {
        out.extend_from_slice(module.join("::").as_bytes());
        out.push(b'\n');
    }
    out
}

fn decode(bytes: &[u8], expected_key: &str) -> Option<Vec<Vec<String>>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    (lines.next()? == CACHE_FORMAT).then_some(())?;
    (lines.next()? == expected_key).then_some(())?;
    let mut mods = Vec::new();
    for line in lines {
        if line.is_empty() {
            return None;
        }
        let parts = line.split("::").map(str::to_owned).collect::<Vec<_>>();
        if parts.is_empty() || parts.iter().any(|part| !is_identifier(part)) {
            return None;
        }
        if mods.iter().any(|existing| existing == &parts) {
            return None;
        }
        mods.push(parts);
    }
    Some(mods)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_changes_with_content() {
        assert_ne!(cache_key(b"route a"), cache_key(b"route b"));
    }

    #[test]
    fn decoder_rejects_wrong_key_and_bad_module() {
        let key = cache_key(b"x");
        assert!(decode(&encode(&key, &[vec!["api".into()]]), &key).is_some());
        assert!(decode(&encode("wrong", &[vec!["api".into()]]), &key).is_none());
        let bad = format!("{CACHE_FORMAT}\n{key}\n../escape\n");
        assert!(decode(bad.as_bytes(), &key).is_none());
    }
}
