use super::error::MigrationError;
use super::types::AppliedMigration;
use crate::DbBackend;
use sqlx::any::install_default_drivers;
use sqlx::{AnyConnection, AssertSqlSafe, Connection, Row};
use std::sync::Once;

static INSTALL_DRIVERS: Once = Once::new();
const STATE_TABLE: &str = "_velran_migrations";

pub(crate) async fn connect(
    url: &str,
    allow_insecure_db: bool,
) -> Result<AnyConnection, MigrationError> {
    INSTALL_DRIVERS.call_once(install_default_drivers);
    let backend = DbBackend::from_url(url).map_err(|_| MigrationError::UnsupportedUrl)?;
    if !backend.compiled_in() {
        return Err(MigrationError::UnsupportedUrl);
    }
    validate_transport(backend, url, allow_insecure_db)?;
    let normalized = if let Some(rest) = url.strip_prefix("mariadb://") {
        format!("mysql://{rest}")
    } else {
        url.to_string()
    };
    AnyConnection::connect(&normalized)
        .await
        .map_err(MigrationError::Sqlx)
}

pub(crate) fn validate_transport(
    backend: DbBackend,
    url: &str,
    allow: bool,
) -> Result<(), MigrationError> {
    if allow || backend == DbBackend::Sqlite {
        return Ok(());
    }
    let lower = url.to_ascii_lowercase();
    let local =
        lower.contains("@localhost") || lower.contains("@127.0.0.1") || lower.contains("@[::1]");
    if local {
        return Ok(());
    }
    let tls = match backend {
        DbBackend::PostgreSql => {
            lower.contains("sslmode=require")
                || lower.contains("sslmode=verify-ca")
                || lower.contains("sslmode=verify-full")
        }
        DbBackend::MariaDb => {
            lower.contains("ssl-mode=required")
                || lower.contains("ssl-mode=verify_ca")
                || lower.contains("ssl-mode=verify_identity")
        }
        DbBackend::Sqlite => true,
    };
    if tls {
        Ok(())
    } else {
        Err(MigrationError::InsecureRemoteDb)
    }
}

pub(crate) async fn state_table_exists(
    conn: &mut AnyConnection,
    backend: DbBackend,
) -> Result<bool, MigrationError> {
    match backend {
        DbBackend::Sqlite => {
            let row = sqlx::query(AssertSqlSafe(
                "SELECT COUNT(*) AS n FROM sqlite_master WHERE type='table' AND name=?".to_string(),
            ))
            .bind(STATE_TABLE)
            .fetch_one(&mut *conn)
            .await?;
            Ok(row.try_get::<i64, _>("n")? > 0)
        }
        DbBackend::PostgreSql => {
            let row = sqlx::query(AssertSqlSafe("SELECT COUNT(*) AS n FROM information_schema.tables WHERE table_schema=current_schema() AND table_name=$1".to_string())).bind(STATE_TABLE).fetch_one(&mut *conn).await?;
            Ok(row.try_get::<i64, _>("n")? > 0)
        }
        DbBackend::MariaDb => {
            let row = sqlx::query(AssertSqlSafe("SELECT COUNT(*) AS n FROM information_schema.tables WHERE table_schema=DATABASE() AND table_name=?".to_string())).bind(STATE_TABLE).fetch_one(&mut *conn).await?;
            Ok(row.try_get::<i64, _>("n")? > 0)
        }
    }
}

pub(crate) async fn ensure_state_table(conn: &mut AnyConnection) -> Result<(), MigrationError> {
    exec_raw(conn, &format!("CREATE TABLE IF NOT EXISTS {STATE_TABLE} (version BIGINT PRIMARY KEY, name TEXT NOT NULL, checksum TEXT NOT NULL, applied_at TEXT NOT NULL)")).await
}

pub(crate) async fn load_applied(
    conn: &mut AnyConnection,
) -> Result<Vec<AppliedMigration>, MigrationError> {
    let sql =
        format!("SELECT version,name,checksum,applied_at FROM {STATE_TABLE} ORDER BY version");
    let rows = sqlx::query(AssertSqlSafe(sql))
        .fetch_all(&mut *conn)
        .await?;
    rows.into_iter()
        .map(|row| {
            Ok(AppliedMigration {
                version: row.try_get("version")?,
                name: row.try_get("name")?,
                checksum: row.try_get("checksum")?,
                applied_at: row.try_get("applied_at")?,
            })
        })
        .collect()
}

pub(crate) async fn exec_statement(conn: &mut AnyConnection, sql: &str) -> Result<(), sqlx::Error> {
    sqlx::query(AssertSqlSafe(sql.to_string()))
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub(crate) async fn exec_raw(conn: &mut AnyConnection, sql: &str) -> Result<(), MigrationError> {
    sqlx::query(AssertSqlSafe(sql.to_string()))
        .execute(&mut *conn)
        .await?;
    Ok(())
}
