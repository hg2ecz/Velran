use super::error::MigrationError;
use super::types::Migration;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

const MAX_MIGRATION_BYTES: u64 = 4 * 1024 * 1024;
const MAX_MIGRATIONS: usize = 10_000;

pub fn load_migrations(dir: &Path) -> Result<Vec<Migration>, MigrationError> {
    let dir_meta = std::fs::symlink_metadata(dir)?;
    if dir_meta.file_type().is_symlink() || !dir_meta.is_dir() {
        return Err(MigrationError::UnsafeMigrationDirectory);
    }
    let mut by_version = BTreeMap::new();
    let mut count = 0usize;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("sql") {
            continue;
        }
        let ty = entry.file_type()?;
        if ty.is_symlink() || !ty.is_file() {
            return Err(MigrationError::UnsafeMigrationFile(
                path.display().to_string(),
            ));
        }
        let meta = entry.metadata()?;
        if meta.len() > MAX_MIGRATION_BYTES {
            return Err(MigrationError::MigrationTooLarge(
                path.display().to_string(),
            ));
        }
        count += 1;
        if count > MAX_MIGRATIONS {
            return Err(MigrationError::TooManyMigrations);
        }
        let filename = path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| MigrationError::InvalidFilename(path.display().to_string()))?;
        let stem = filename.strip_suffix(".sql").unwrap();
        let (version_s, name) = stem
            .split_once('_')
            .ok_or_else(|| MigrationError::InvalidFilename(filename.into()))?;
        if version_s.is_empty()
            || !version_s.bytes().all(|b| b.is_ascii_digit())
            || name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(MigrationError::InvalidFilename(filename.into()));
        }
        let version: i64 = version_s
            .parse()
            .map_err(|_| MigrationError::InvalidFilename(filename.into()))?;
        if version <= 0 {
            return Err(MigrationError::InvalidVersion);
        }
        let sql = std::fs::read_to_string(&path)?;
        let statements = split_sql_statements(&sql);
        if statements.is_empty() {
            return Err(MigrationError::EmptyMigration { version });
        }
        let checksum = hex_sha256(sql.as_bytes());
        let migration = Migration {
            version,
            name: name.into(),
            path,
            checksum,
            sql,
        };
        if by_version.insert(version, migration).is_some() {
            return Err(MigrationError::DuplicateVersion(version));
        }
    }
    let migrations: Vec<_> = by_version.into_values().collect();
    if migrations.windows(2).any(|w| w[0].version >= w[1].version) {
        return Err(MigrationError::InvalidVersion);
    }
    Ok(migrations)
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn split_sql_statements(sql: &str) -> Vec<String> {
    let bytes = sql.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    let mut single = false;
    let mut double = false;
    let mut backtick = false;
    let mut line_comment = false;
    let mut block_comment = false;
    let mut dollar: Option<Vec<u8>> = None;
    while i < bytes.len() {
        if let Some(delim) = dollar.as_ref() {
            if bytes[i..].starts_with(delim) {
                i += delim.len();
                dollar = None;
            } else {
                i += 1;
            }
            continue;
        }
        if line_comment {
            if bytes[i] == b'\n' {
                line_comment = false;
            }
            i += 1;
            continue;
        }
        if block_comment {
            if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                block_comment = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if !single
            && !double
            && !backtick
            && i + 1 < bytes.len()
            && bytes[i] == b'-'
            && bytes[i + 1] == b'-'
        {
            line_comment = true;
            i += 2;
            continue;
        }
        if !single
            && !double
            && !backtick
            && i + 1 < bytes.len()
            && bytes[i] == b'/'
            && bytes[i + 1] == b'*'
        {
            block_comment = true;
            i += 2;
            continue;
        }
        if !single && !double && !backtick && bytes[i] == b'$' {
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'$' {
                dollar = Some(bytes[i..=j].to_vec());
                i = j + 1;
                continue;
            }
        }
        if bytes[i] == b'\'' && !double && !backtick {
            if single && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                i += 2;
                continue;
            }
            single = !single;
            i += 1;
            continue;
        }
        if bytes[i] == b'"' && !single && !backtick {
            if double && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                i += 2;
                continue;
            }
            double = !double;
            i += 1;
            continue;
        }
        if bytes[i] == b'`' && !single && !double {
            backtick = !backtick;
            i += 1;
            continue;
        }
        if bytes[i] == b';' && !single && !double && !backtick {
            let stmt = sql[start..i].trim();
            if !stmt.is_empty() {
                out.push(stmt.to_string());
            }
            start = i + 1;
        }
        i += 1;
    }
    let tail = sql[start..].trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out
}
