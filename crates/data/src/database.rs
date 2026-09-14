use crate::{
    BindSet, DataError, DbBackend, DbConfig, DbRow, DbScalarType, DbValue, ExecuteResult,
    PreparedSql, RowShape,
};
use sqlx::any::{AnyPoolOptions, install_default_drivers};
use sqlx::{Any, AnyPool, AssertSqlSafe, Column, Row, Transaction};
use std::collections::{HashMap, HashSet};
use std::sync::Once;
use std::time::Duration;

static INSTALL_SQLX_DRIVERS: Once = Once::new();

#[derive(Clone)]
pub struct Database {
    backend: DbBackend,
    pool: AnyPool,
    query_timeout: Duration,
    max_rows: usize,
    max_result_bytes: usize,
}

impl Database {
    pub async fn connect(config: DbConfig) -> Result<Self, DataError> {
        INSTALL_SQLX_DRIVERS.call_once(install_default_drivers);
        let backend = DbBackend::from_url(&config.url)?;
        if !backend.compiled_in() {
            return Err(DataError::UnsupportedDatabaseScheme);
        }
        validate_db_transport(backend, &config.url, config.require_tls_for_remote)?;
        let pool = AnyPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.acquire_timeout)
            .connect(&normalize_mariadb_url(&config.url))
            .await
            .map_err(DataError::Sqlx)?;
        Ok(Self {
            backend,
            pool,
            query_timeout: config.query_timeout,
            max_rows: config.max_rows,
            max_result_bytes: config.max_result_bytes,
        })
    }

    pub fn backend(&self) -> DbBackend {
        self.backend
    }

    pub async fn ping(&self) -> Result<(), DataError> {
        let sql = PreparedSql::compile("SELECT 1")?;
        self.execute(&sql, &BindSet::new()).await.map(|_| ())
    }

    pub async fn execute(
        &self,
        sql: &PreparedSql,
        binds: &BindSet,
    ) -> Result<ExecuteResult, DataError> {
        let rendered = sql.render_for(self.backend)?;
        let ordered = binds.ordered(sql)?;
        let mut query = sqlx::query(AssertSqlSafe(rendered));
        for value in ordered {
            query = bind_any(query, value);
        }
        let result = tokio::time::timeout(self.query_timeout, query.execute(&self.pool))
            .await
            .map_err(|_| DataError::Timeout)?
            .map_err(DataError::Sqlx)?;
        Ok(ExecuteResult {
            rows_affected: result.rows_affected(),
        })
    }

    pub async fn fetch_all(
        &self,
        sql: &PreparedSql,
        binds: &BindSet,
        shape: &RowShape,
    ) -> Result<Vec<DbRow>, DataError> {
        validate_row_shape(shape)?;
        let rendered = sql.render_for(self.backend)?;
        let ordered = binds.ordered(sql)?;
        let mut query = sqlx::query(AssertSqlSafe(rendered));
        for value in ordered {
            query = bind_any(query, value);
        }
        let rows = tokio::time::timeout(self.query_timeout, query.fetch_all(&self.pool))
            .await
            .map_err(|_| DataError::Timeout)?
            .map_err(DataError::Sqlx)?;
        if rows.len() > self.max_rows {
            return Err(DataError::RowLimitExceeded);
        }
        decode_rows(rows, shape, self.max_result_bytes)
    }

    pub async fn begin(&self) -> Result<DbTransaction<'_>, DataError> {
        let tx = tokio::time::timeout(self.query_timeout, self.pool.begin())
            .await
            .map_err(|_| DataError::Timeout)?
            .map_err(DataError::Sqlx)?;
        Ok(DbTransaction {
            backend: self.backend,
            tx,
            query_timeout: self.query_timeout,
            max_rows: self.max_rows,
            max_result_bytes: self.max_result_bytes,
        })
    }
}

pub struct DbTransaction<'a> {
    backend: DbBackend,
    tx: Transaction<'a, Any>,
    query_timeout: Duration,
    max_rows: usize,
    max_result_bytes: usize,
}

impl<'a> DbTransaction<'a> {
    pub async fn execute(
        &mut self,
        sql: &PreparedSql,
        binds: &BindSet,
    ) -> Result<ExecuteResult, DataError> {
        let rendered = sql.render_for(self.backend)?;
        let ordered = binds.ordered(sql)?;
        let mut query = sqlx::query(AssertSqlSafe(rendered));
        for value in ordered {
            query = bind_any(query, value);
        }
        let result = tokio::time::timeout(self.query_timeout, query.execute(&mut *self.tx))
            .await
            .map_err(|_| DataError::Timeout)?
            .map_err(DataError::Sqlx)?;
        Ok(ExecuteResult {
            rows_affected: result.rows_affected(),
        })
    }

    pub async fn fetch_all(
        &mut self,
        sql: &PreparedSql,
        binds: &BindSet,
        shape: &RowShape,
    ) -> Result<Vec<DbRow>, DataError> {
        validate_row_shape(shape)?;
        let rendered = sql.render_for(self.backend)?;
        let ordered = binds.ordered(sql)?;
        let mut query = sqlx::query(AssertSqlSafe(rendered));
        for value in ordered {
            query = bind_any(query, value);
        }
        let rows = tokio::time::timeout(self.query_timeout, query.fetch_all(&mut *self.tx))
            .await
            .map_err(|_| DataError::Timeout)?
            .map_err(DataError::Sqlx)?;
        if rows.len() > self.max_rows {
            return Err(DataError::RowLimitExceeded);
        }
        decode_rows(rows, shape, self.max_result_bytes)
    }

    pub async fn commit(self) -> Result<(), DataError> {
        self.tx.commit().await.map_err(DataError::Sqlx)
    }

    pub async fn rollback(self) -> Result<(), DataError> {
        self.tx.rollback().await.map_err(DataError::Sqlx)
    }
}

fn validate_row_shape(shape: &RowShape) -> Result<(), DataError> {
    if shape.columns.is_empty() {
        return Err(DataError::InvalidRowShape);
    }
    let mut seen = HashSet::new();
    for column in &shape.columns {
        if column.name.is_empty() || !seen.insert(column.name.as_str()) {
            return Err(DataError::InvalidRowShape);
        }
    }
    Ok(())
}

fn decode_rows(
    rows: Vec<sqlx::any::AnyRow>,
    shape: &RowShape,
    max_bytes: usize,
) -> Result<Vec<DbRow>, DataError> {
    let mut output = Vec::with_capacity(rows.len());
    let mut bytes_used = 0usize;
    for row in rows {
        if row.columns().len() != shape.columns.len() {
            return Err(DataError::RowShapeMismatch);
        }
        let mut values = HashMap::with_capacity(shape.columns.len());
        for (index, spec) in shape.columns.iter().enumerate() {
            let actual_name = row.columns()[index].name();
            if actual_name != spec.name {
                return Err(DataError::RowShapeMismatch);
            }
            let value = match spec.ty {
                DbScalarType::String => {
                    DbValue::String(row.try_get::<String, _>(index).map_err(DataError::Sqlx)?)
                }
                DbScalarType::Int => {
                    DbValue::Int(row.try_get::<i64, _>(index).map_err(DataError::Sqlx)?)
                }
                DbScalarType::Bool => {
                    DbValue::Bool(row.try_get::<bool, _>(index).map_err(DataError::Sqlx)?)
                }
                DbScalarType::Bytes => {
                    DbValue::Bytes(row.try_get::<Vec<u8>, _>(index).map_err(DataError::Sqlx)?)
                }
            };
            bytes_used = bytes_used
                .saturating_add(spec.name.len())
                .saturating_add(db_value_size(&value));
            if bytes_used > max_bytes {
                return Err(DataError::ResultSizeLimitExceeded);
            }
            values.insert(spec.name.clone(), value);
        }
        output.push(DbRow { values });
    }
    Ok(output)
}

fn db_value_size(value: &DbValue) -> usize {
    match value {
        DbValue::String(v) => v.len(),
        DbValue::Int(_) => std::mem::size_of::<i64>(),
        DbValue::Bool(_) => 1,
        DbValue::Bytes(v) => v.len(),
    }
}

fn bind_any<'q>(
    query: sqlx::query::Query<'q, Any, <Any as sqlx::Database>::Arguments>,
    value: &'q DbValue,
) -> sqlx::query::Query<'q, Any, <Any as sqlx::Database>::Arguments> {
    match value {
        DbValue::String(v) => query.bind(v.as_str()),
        DbValue::Int(v) => query.bind(*v),
        DbValue::Bool(v) => query.bind(*v),
        DbValue::Bytes(v) => query.bind(v.as_slice()),
    }
}

fn normalize_mariadb_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("mariadb://") {
        format!("mysql://{rest}")
    } else {
        url.to_string()
    }
}

fn validate_db_transport(
    backend: DbBackend,
    url: &str,
    require_tls: bool,
) -> Result<(), DataError> {
    if !require_tls || backend == DbBackend::Sqlite {
        return Ok(());
    }
    // SQLx backend-specific TLS URL options vary. We intentionally require an explicit
    // TLS/encryption marker instead of silently accepting plaintext remote DB URLs.
    let lower = url.to_ascii_lowercase();
    let tls_marker = lower.contains("sslmode=require")
        || lower.contains("sslmode=verify-ca")
        || lower.contains("sslmode=verify-full")
        || lower.contains("ssl-mode=required")
        || lower.contains("ssl-mode=verify_ca")
        || lower.contains("ssl-mode=verify_identity")
        || lower.contains("sslmode=required");
    if !tls_marker {
        return Err(DataError::TlsRequired);
    }
    Ok(())
}
