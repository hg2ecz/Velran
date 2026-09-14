use super::error::MigrationError;
use super::types::{AppliedMigration, Migration};
use std::collections::{BTreeMap, HashSet};

pub(crate) fn validate_history(
    local: &[Migration],
    applied: &[AppliedMigration],
) -> Result<(), MigrationError> {
    let local_by_version: BTreeMap<i64, &Migration> =
        local.iter().map(|m| (m.version, m)).collect();
    for old in applied {
        let now = local_by_version
            .get(&old.version)
            .ok_or(MigrationError::MissingLocalMigration(old.version))?;
        if now.name != old.name {
            return Err(MigrationError::NameMismatch {
                version: old.version,
            });
        }
        if now.checksum != old.checksum {
            return Err(MigrationError::ChecksumMismatch {
                version: old.version,
            });
        }
    }
    if let Some(max_applied) = applied.iter().map(|m| m.version).max() {
        let applied_versions: HashSet<i64> = applied.iter().map(|m| m.version).collect();
        if let Some(migration) = local
            .iter()
            .find(|m| m.version < max_applied && !applied_versions.contains(&m.version))
        {
            return Err(MigrationError::OutOfOrderPending {
                version: migration.version,
                max_applied,
            });
        }
    }
    Ok(())
}
