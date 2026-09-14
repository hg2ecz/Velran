use super::database::exec_raw;
use super::error::MigrationError;
use crate::DbBackend;
use sqlx::{AnyConnection, AssertSqlSafe, Row};
use std::time::Duration;

const LOCK_NAME: &str = "velran_migrations_v1";
const PG_LOCK_KEY: i64 = 0x5257_4d49_4752_4154;

pub(crate) async fn acquire_lock(
    conn: &mut AnyConnection,
    backend: DbBackend,
    timeout: Duration,
) -> Result<(), MigrationError> {
    match backend {
        DbBackend::PostgreSql => {
            let deadline = tokio::time::Instant::now() + timeout;
            loop {
                let row = sqlx::query(AssertSqlSafe(
                    "SELECT pg_try_advisory_lock($1) AS locked".to_string(),
                ))
                .bind(PG_LOCK_KEY)
                .fetch_one(&mut *conn)
                .await?;
                if row.try_get::<bool, _>("locked")? {
                    return Ok(());
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(MigrationError::LockBusy);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        DbBackend::MariaDb => {
            let secs = timeout.as_secs().min(i64::MAX as u64) as i64;
            let row = sqlx::query(AssertSqlSafe("SELECT GET_LOCK(?, ?) AS locked".to_string()))
                .bind(LOCK_NAME)
                .bind(secs)
                .fetch_one(&mut *conn)
                .await?;
            let locked: Option<i64> = row.try_get("locked")?;
            if locked == Some(1) {
                Ok(())
            } else {
                Err(MigrationError::LockBusy)
            }
        }
        DbBackend::Sqlite => {
            // BEGIN IMMEDIATE takes the SQLite writer lock before any migration statement runs.
            tokio::time::timeout(timeout, exec_raw(conn, "BEGIN IMMEDIATE"))
                .await
                .map_err(|_| MigrationError::LockBusy)??;
            exec_raw(conn, "CREATE TABLE IF NOT EXISTS _velran_migration_lock (id INTEGER PRIMARY KEY, holder TEXT NOT NULL)").await?;
            exec_raw(
                conn,
                "INSERT OR IGNORE INTO _velran_migration_lock(id,holder) VALUES (1,'velran')",
            )
            .await?;
            Ok(())
        }
    }
}

pub(crate) async fn release_lock(
    conn: &mut AnyConnection,
    backend: DbBackend,
    success: bool,
) -> Result<(), MigrationError> {
    match backend {
        DbBackend::PostgreSql => {
            let _ = sqlx::query(AssertSqlSafe("SELECT pg_advisory_unlock($1)".to_string()))
                .bind(PG_LOCK_KEY)
                .execute(&mut *conn)
                .await?;
        }
        DbBackend::MariaDb => {
            let _ = sqlx::query(AssertSqlSafe("SELECT RELEASE_LOCK(?)".to_string()))
                .bind(LOCK_NAME)
                .execute(&mut *conn)
                .await?;
        }
        DbBackend::Sqlite => {
            exec_raw(conn, if success { "COMMIT" } else { "ROLLBACK" }).await?;
        }
    }
    Ok(())
}
