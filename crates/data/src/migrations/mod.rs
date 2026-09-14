mod database;
mod error;
mod error_display;
mod history;
mod locking;
mod service;
mod source;
mod types;

pub use error::MigrationError;
pub use service::{apply, status, verify};
pub use source::{load_migrations, split_sql_statements};
pub use types::{AppliedMigration, Migration, MigrationState, MigrationStatus};

#[cfg(test)]
mod tests;
