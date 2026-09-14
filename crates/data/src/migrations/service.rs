use super::database::{
    connect, ensure_state_table, exec_raw, exec_statement, load_applied, state_table_exists,
};
use super::error::MigrationError;
use super::history::validate_history;
use super::locking::{acquire_lock, release_lock};
use super::source::{load_migrations, split_sql_statements};
use super::types::{Migration, MigrationState, MigrationStatus};
use crate::DbBackend;
use sqlx::{AnyConnection, AssertSqlSafe};
use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STATE_TABLE: &str = "_velran_migrations";

pub async fn status(
    url: &str,
    dir: &Path,
    allow_insecure_db: bool,
) -> Result<Vec<MigrationStatus>, MigrationError> {
    let local = load_migrations(dir)?;
    let mut conn = connect(url, allow_insecure_db).await?;
    let backend = DbBackend::from_url(url).map_err(|_| MigrationError::UnsupportedUrl)?;
    let applied = if state_table_exists(&mut conn, backend).await? {
        load_applied(&mut conn).await?
    } else {
        Vec::new()
    };
    validate_history(&local, &applied)?;
    let applied_versions: HashSet<i64> = applied.into_iter().map(|m| m.version).collect();
    Ok(local
        .into_iter()
        .map(|m| MigrationStatus {
            version: m.version,
            name: m.name,
            state: if applied_versions.contains(&m.version) {
                MigrationState::Applied
            } else {
                MigrationState::Pending
            },
        })
        .collect())
}

pub async fn verify(url: &str, dir: &Path, allow_insecure_db: bool) -> Result<(), MigrationError> {
    let local = load_migrations(dir)?;
    let mut conn = connect(url, allow_insecure_db).await?;
    let backend = DbBackend::from_url(url).map_err(|_| MigrationError::UnsupportedUrl)?;
    let applied = if state_table_exists(&mut conn, backend).await? {
        load_applied(&mut conn).await?
    } else {
        Vec::new()
    };
    validate_history(&local, &applied)
}

pub async fn apply(
    url: &str,
    dir: &Path,
    allow_insecure_db: bool,
    lock_timeout: Duration,
) -> Result<Vec<i64>, MigrationError> {
    let local = load_migrations(dir)?;
    let backend = DbBackend::from_url(url).map_err(|_| MigrationError::UnsupportedUrl)?;
    let mut conn = connect(url, allow_insecure_db).await?;
    acquire_lock(&mut conn, backend, lock_timeout).await?;
    let result = apply_locked(&mut conn, backend, &local).await;
    match result {
        Ok(changed) => {
            release_lock(&mut conn, backend, true).await?;
            Ok(changed)
        }
        Err(err) => {
            let _ = release_lock(&mut conn, backend, false).await;
            Err(err)
        }
    }
}

async fn apply_locked(
    conn: &mut AnyConnection,
    backend: DbBackend,
    local: &[Migration],
) -> Result<Vec<i64>, MigrationError> {
    ensure_state_table(conn).await?;
    let applied = load_applied(conn).await?;
    validate_history(local, &applied)?;
    let applied_versions: HashSet<i64> = applied.iter().map(|m| m.version).collect();
    let mut changed = Vec::new();
    for migration in local
        .iter()
        .filter(|m| !applied_versions.contains(&m.version))
    {
        let statements = split_sql_statements(&migration.sql);
        // PostgreSQL and SQLite DDL are applied transactionally here. MariaDB may auto-commit DDL;
        // state is recorded only after every statement succeeds, so a partial MariaDB DDL failure is visible and requires operator repair.
        if backend == DbBackend::PostgreSql {
            exec_raw(conn, "BEGIN").await?;
        }
        let mut failed = None;
        for (idx, stmt) in statements.iter().enumerate() {
            if let Err(source) = exec_statement(conn, stmt).await {
                failed = Some(MigrationError::Statement {
                    version: migration.version,
                    statement: idx + 1,
                    source,
                });
                break;
            }
        }
        if failed.is_none() {
            let applied_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string();
            let insert = match backend {
                DbBackend::PostgreSql => format!(
                    "INSERT INTO {STATE_TABLE}(version,name,checksum,applied_at) VALUES ($1,$2,$3,$4)"
                ),
                _ => format!(
                    "INSERT INTO {STATE_TABLE}(version,name,checksum,applied_at) VALUES (?,?,?,?)"
                ),
            };
            let query = sqlx::query(AssertSqlSafe(insert))
                .bind(migration.version)
                .bind(&migration.name)
                .bind(&migration.checksum)
                .bind(applied_at);
            if let Err(source) = query.execute(&mut *conn).await {
                failed = Some(MigrationError::Statement {
                    version: migration.version,
                    statement: statements.len() + 1,
                    source,
                });
            }
        }
        if let Some(err) = failed {
            if backend == DbBackend::PostgreSql {
                let _ = exec_raw(conn, "ROLLBACK").await;
            }
            return Err(err);
        }
        if backend == DbBackend::PostgreSql {
            exec_raw(conn, "COMMIT").await?;
        }
        changed.push(migration.version);
    }
    Ok(changed)
}
