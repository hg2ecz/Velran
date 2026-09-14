use crate::DataError;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbBackend {
    Sqlite,
    PostgreSql,
    MariaDb,
}

impl DbBackend {
    pub fn from_url(url: &str) -> Result<Self, DataError> {
        if url.starts_with("sqlite:") {
            Ok(Self::Sqlite)
        } else if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(Self::PostgreSql)
        } else if url.starts_with("mysql://") || url.starts_with("mariadb://") {
            Ok(Self::MariaDb)
        } else {
            Err(DataError::UnsupportedDatabaseScheme)
        }
    }

    pub fn compiled_in(self) -> bool {
        match self {
            Self::Sqlite => cfg!(feature = "db-sqlite"),
            Self::PostgreSql => cfg!(feature = "db-postgres"),
            Self::MariaDb => cfg!(feature = "db-mysql"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub query_timeout: Duration,
    pub max_rows: usize,
    pub max_result_bytes: usize,
    pub require_tls_for_remote: bool,
}

impl DbConfig {
    pub fn secure_default(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            max_connections: 32,
            min_connections: 0,
            acquire_timeout: Duration::from_secs(5),
            query_timeout: Duration::from_secs(10),
            max_rows: 10_000,
            max_result_bytes: 16 * 1024 * 1024,
            require_tls_for_remote: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DbValue {
    String(String),
    Int(i64),
    Bool(bool),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbScalarType {
    String,
    Int,
    Bool,
    Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSpec {
    pub name: String,
    pub ty: DbScalarType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowShape {
    pub columns: Vec<ColumnSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DbRow {
    pub(crate) values: HashMap<String, DbValue>,
}

impl DbRow {
    pub fn get(&self, name: &str) -> Option<&DbValue> {
        self.values.get(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecuteResult {
    pub rows_affected: u64,
}
