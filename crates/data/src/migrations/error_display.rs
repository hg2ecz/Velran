use super::error::MigrationError;
use std::fmt;

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Sqlx(error) => write!(f, "database error: {error}"),
            Self::UnsupportedUrl => f.write_str("unsupported database URL"),
            Self::InsecureRemoteDb => f.write_str("remote migration database requires TLS; use --allow-insecure-db only for an explicit trusted exception"),
            Self::InvalidFilename(name) => write!(f, "invalid migration filename `{name}`; expected NNNN_name.sql"),
            Self::UnsafeMigrationDirectory => f.write_str("migration directory must be a real directory, not a symlink"),
            Self::UnsafeMigrationFile(name) => write!(f, "migration `{name}` must be a regular non-symlink file"),
            Self::MigrationTooLarge(name) => write!(f, "migration `{name}` exceeds the 4 MiB file limit"),
            Self::TooManyMigrations => f.write_str("too many migration files"),
            Self::DuplicateVersion(version) => write!(f, "duplicate migration version {version}"),
            Self::InvalidVersion => f.write_str("migration versions must be strictly increasing positive integers"),
            Self::ChecksumMismatch { version } => write!(f, "migration {version} checksum changed after it was applied"),
            Self::NameMismatch { version } => write!(f, "migration {version} name changed after it was applied"),
            Self::MissingLocalMigration(version) => write!(f, "database contains applied migration {version} that is missing locally"),
            Self::OutOfOrderPending { version, max_applied } => write!(f, "pending migration {version} is older than already applied migration {max_applied}; renumber it instead of inserting history"),
            Self::LockBusy => f.write_str("migration lock is already held"),
            Self::EmptyMigration { version } => write!(f, "migration {version} contains no executable SQL"),
            Self::Statement { version, statement, source } => write!(f, "migration {version} failed at statement {statement}: {source}"),
        }
    }
}
