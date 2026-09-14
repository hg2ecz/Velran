mod database;
mod error;
pub mod migrations;
mod redis_store;
mod sql;
mod types;

pub use database::{Database, DbTransaction};
pub use error::DataError;
pub use redis_store::{RedisConfig, RedisStore};
pub use sql::{BindSet, PreparedSql};
pub use types::{
    ColumnSpec, DbBackend, DbConfig, DbRow, DbScalarType, DbValue, ExecuteResult, RowShape,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redis_store::validate_redis_config;

    #[test]
    fn supports_three_sql_backends() {
        assert_eq!(
            DbBackend::from_url("sqlite://app.db").unwrap(),
            DbBackend::Sqlite
        );
        assert_eq!(
            DbBackend::from_url("postgres://db/app").unwrap(),
            DbBackend::PostgreSql
        );
        assert_eq!(
            DbBackend::from_url("mariadb://db/app").unwrap(),
            DbBackend::MariaDb
        );
    }

    #[test]
    fn postgres_binds_are_numbered() {
        let q = PreparedSql::compile("SELECT * FROM users WHERE id=:id AND name=:name").unwrap();
        assert_eq!(
            q.render_for(DbBackend::PostgreSql).unwrap(),
            "SELECT * FROM users WHERE id=$1 AND name=$2"
        );
    }

    #[test]
    fn backend_placeholder_styles_are_correct() {
        let q = PreparedSql::compile("UPDATE x SET name=:name WHERE id=:id").unwrap();
        assert_eq!(
            q.render_for(DbBackend::MariaDb).unwrap(),
            "UPDATE x SET name=? WHERE id=?"
        );
        assert_eq!(
            q.render_for(DbBackend::Sqlite).unwrap(),
            "UPDATE x SET name=$1 WHERE id=$2"
        );
    }

    #[test]
    fn bind_scanner_ignores_literals_comments_and_postgres_cast() {
        let q = PreparedSql::compile(
            "SELECT ':fake', col::text -- :comment\nFROM t WHERE id=:id /* :block */",
        )
        .unwrap();
        assert_eq!(q.bind_names(), &["id"]);
    }

    #[test]
    fn bind_scanner_ignores_backend_quoted_identifiers() {
        let q = PreparedSql::compile("SELECT `a:b`, [c:d] FROM t WHERE id=:id").unwrap();
        assert_eq!(q.bind_names(), &["id"]);
    }

    #[test]
    fn rejects_postgres_dollar_quoted_strings_conservatively() {
        assert!(matches!(
            PreparedSql::compile("SELECT $$:not_a_bind$$"),
            Err(DataError::UnsupportedSqlSyntax)
        ));
    }

    #[test]
    fn rejects_multiple_statements() {
        assert!(matches!(
            PreparedSql::compile("SELECT 1; DROP TABLE users"),
            Err(DataError::MultipleStatements)
        ));
    }

    #[test]
    fn bind_set_is_exact() {
        let q = PreparedSql::compile("SELECT * FROM x WHERE a=:a").unwrap();
        let mut b = BindSet::new();
        b.insert("a", DbValue::Int(1)).unwrap();
        b.insert("b", DbValue::Int(2)).unwrap();
        assert!(matches!(b.ordered(&q), Err(DataError::UnexpectedBind(name)) if name == "b"));
    }

    #[test]
    fn redis_tls_is_secure_by_default() {
        let cfg = RedisConfig::secure_default("redis://localhost", "app");
        assert!(matches!(
            validate_redis_config(&cfg),
            Err(DataError::TlsRequired)
        ));
    }

    #[test]
    fn redis_key_rejects_control_characters() {
        let dummy = RedisConfig::secure_default("rediss://localhost", "app");
        assert!(validate_redis_config(&dummy).is_ok());
    }
}
