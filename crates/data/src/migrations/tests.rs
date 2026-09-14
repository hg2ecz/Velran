use super::error::MigrationError;
use super::history::validate_history;
use super::types::{AppliedMigration, Migration, MigrationState};
use super::{apply, load_migrations, split_sql_statements, status, verify};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn loader_orders_and_hashes() {
    let dir = std::env::temp_dir().join(format!("velran-migrations-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("0002_add_index.sql"), "CREATE INDEX x ON t(id);\n").unwrap();
    fs::write(
        dir.join("0001_init.sql"),
        "CREATE TABLE t(id BIGINT PRIMARY KEY);\n",
    )
    .unwrap();
    let m = load_migrations(&dir).unwrap();
    assert_eq!(m.iter().map(|m| m.version).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(m[0].checksum.len(), 64);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn splitter_ignores_semicolons_in_strings_and_comments() {
    let s = "CREATE TABLE t(v TEXT); INSERT INTO t VALUES ('a;b'); -- ;\n/* ; */ UPDATE t SET v=\"x;y\";";
    assert_eq!(split_sql_statements(s).len(), 3);
}

#[test]
fn splitter_preserves_postgres_dollar_quoted_body() {
    let s = "CREATE FUNCTION f() RETURNS void AS $$ BEGIN PERFORM 1; PERFORM 2; END $$ LANGUAGE plpgsql; SELECT 1;";
    assert_eq!(split_sql_statements(s).len(), 2);
}

#[test]
fn history_rejects_backfilled_old_version() {
    let local = vec![
        Migration {
            version: 1,
            name: "one".into(),
            path: PathBuf::new(),
            checksum: "a".into(),
            sql: "SELECT 1".into(),
        },
        Migration {
            version: 2,
            name: "two".into(),
            path: PathBuf::new(),
            checksum: "b".into(),
            sql: "SELECT 2".into(),
        },
        Migration {
            version: 3,
            name: "three".into(),
            path: PathBuf::new(),
            checksum: "c".into(),
            sql: "SELECT 3".into(),
        },
    ];
    let applied = vec![
        AppliedMigration {
            version: 1,
            name: "one".into(),
            checksum: "a".into(),
            applied_at: "0".into(),
        },
        AppliedMigration {
            version: 3,
            name: "three".into(),
            checksum: "c".into(),
            applied_at: "0".into(),
        },
    ];
    assert!(matches!(
        validate_history(&local, &applied),
        Err(MigrationError::OutOfOrderPending {
            version: 2,
            max_applied: 3
        })
    ));
}

#[tokio::test]
async fn sqlite_apply_status_verify_roundtrip() {
    let base = std::env::temp_dir().join(format!("velran-migration-it-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let dir = base.join("migrations");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("0001_init.sql"),"CREATE TABLE thing(id BIGINT PRIMARY KEY, name TEXT NOT NULL); INSERT INTO thing(id,name) VALUES (1,'ok');").unwrap();
    let db = base.join("db.sqlite");
    let url = format!("sqlite://{}?mode=rwc", db.display());
    let changed = apply(&url, &dir, false, Duration::from_secs(2))
        .await
        .unwrap();
    assert_eq!(changed, vec![1]);
    verify(&url, &dir, false).await.unwrap();
    let st = status(&url, &dir, false).await.unwrap();
    assert_eq!(st.len(), 1);
    assert_eq!(st[0].state, MigrationState::Applied);
    let changed = apply(&url, &dir, false, Duration::from_secs(2))
        .await
        .unwrap();
    assert!(changed.is_empty());
    let _ = fs::remove_dir_all(base);
}
