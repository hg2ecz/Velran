#[derive(Debug)]
pub enum MigrationError {
    Io(std::io::Error),
    Sqlx(sqlx::Error),
    UnsupportedUrl,
    InsecureRemoteDb,
    InvalidFilename(String),
    UnsafeMigrationDirectory,
    UnsafeMigrationFile(String),
    MigrationTooLarge(String),
    TooManyMigrations,
    DuplicateVersion(i64),
    InvalidVersion,
    ChecksumMismatch {
        version: i64,
    },
    NameMismatch {
        version: i64,
    },
    MissingLocalMigration(i64),
    OutOfOrderPending {
        version: i64,
        max_applied: i64,
    },
    LockBusy,
    EmptyMigration {
        version: i64,
    },
    Statement {
        version: i64,
        statement: usize,
        source: sqlx::Error,
    },
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sqlx(error) => Some(error),
            Self::Statement { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for MigrationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<sqlx::Error> for MigrationError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}
