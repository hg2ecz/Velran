use crate::{AuthError, TenantId, random_hex};
use std::time::Duration;

// --- M25 local authentication -------------------------------------------------
use argon2::password_hash::SaltString;
use argon2::{Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier};
use data::{
    BindSet, ColumnSpec, Database, DbBackend, DbConfig, DbScalarType, DbValue, PreparedSql,
    RowShape,
};
use sha2::{Digest, Sha256};

mod memberships;

#[derive(Debug, Clone)]
pub struct LocalUserAuth {
    pub username: String,
    pub roles: Vec<String>,
    pub memberships: Vec<TenantId>,
    pub totp_secret: Option<Vec<u8>>,
    pub auth_generation: u64,
}

#[derive(Clone)]
pub struct LocalUserStore {
    db: Database,
}

impl LocalUserStore {
    pub async fn connect_sqlite(url: &str) -> Result<Self, AuthError> {
        if DbBackend::from_url(url).map_err(|_| AuthError::StoreUnavailable)? != DbBackend::Sqlite {
            return Err(AuthError::StoreUnavailable);
        }
        let mut cfg = DbConfig::secure_default(url.to_string());
        cfg.max_connections = 8;
        cfg.query_timeout = Duration::from_secs(5);
        let db = Database::connect(cfg)
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        Ok(Self { db })
    }

    pub async fn initialize(&self) -> Result<(), AuthError> {
        for sql in [
            "CREATE TABLE IF NOT EXISTS _velran_local_users (username TEXT PRIMARY KEY, password_hash TEXT NOT NULL, disabled INTEGER NOT NULL DEFAULT 0, totp_secret BLOB NULL, auth_generation INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, password_changed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)",
            "CREATE TABLE IF NOT EXISTS _velran_local_user_roles (username TEXT NOT NULL, role TEXT NOT NULL, PRIMARY KEY(username, role), FOREIGN KEY(username) REFERENCES _velran_local_users(username) ON DELETE CASCADE)",
            "CREATE TABLE IF NOT EXISTS _velran_local_user_memberships (username TEXT NOT NULL, tenant_id TEXT NOT NULL, PRIMARY KEY(username, tenant_id), FOREIGN KEY(username) REFERENCES _velran_local_users(username) ON DELETE CASCADE)",
            "CREATE TABLE IF NOT EXISTS _velran_local_recovery_codes (username TEXT NOT NULL, code_hash TEXT NOT NULL, PRIMARY KEY(username, code_hash), FOREIGN KEY(username) REFERENCES _velran_local_users(username) ON DELETE CASCADE)",
        ] {
            self.db
                .execute(
                    &PreparedSql::compile(sql).map_err(|_| AuthError::Internal)?,
                    &BindSet::new(),
                )
                .await
                .map_err(|_| AuthError::StoreUnavailable)?;
        }
        Ok(())
    }

    pub async fn create_user(
        &self,
        username: &str,
        password: &str,
        roles: &[String],
    ) -> Result<(), AuthError> {
        let username = canonical_local_username(username)?;
        validate_password(password)?;
        validate_roles(roles)?;
        let hash = hash_password(password)?;
        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username.clone()))
            .map_err(|_| AuthError::Internal)?;
        b.insert("hash", DbValue::String(hash))
            .map_err(|_| AuthError::Internal)?;
        tx.execute(&PreparedSql::compile("INSERT INTO _velran_local_users(username,password_hash,disabled) VALUES(:username,:hash,0)").map_err(|_|AuthError::Internal)?,&b).await.map_err(|_|AuthError::InvalidCredentials)?;
        for role in roles {
            let mut rb = BindSet::new();
            rb.insert("username", DbValue::String(username.clone()))
                .map_err(|_| AuthError::Internal)?;
            rb.insert("role", DbValue::String(role.clone()))
                .map_err(|_| AuthError::Internal)?;
            tx.execute(
                &PreparedSql::compile(
                    "INSERT INTO _velran_local_user_roles(username,role) VALUES(:username,:role)",
                )
                .map_err(|_| AuthError::Internal)?,
                &rb,
            )
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        }
        tx.commit().await.map_err(|_| AuthError::StoreUnavailable)
    }

    pub async fn set_password(&self, username: &str, password: &str) -> Result<(), AuthError> {
        let username = canonical_local_username(username)?;
        validate_password(password)?;
        let hash = hash_password(password)?;
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username))
            .map_err(|_| AuthError::Internal)?;
        b.insert("hash", DbValue::String(hash))
            .map_err(|_| AuthError::Internal)?;
        let r=self.db.execute(&PreparedSql::compile("UPDATE _velran_local_users SET password_hash=:hash,password_changed_at=CURRENT_TIMESTAMP,auth_generation=auth_generation+1 WHERE username=:username").map_err(|_|AuthError::Internal)?,&b).await.map_err(|_|AuthError::StoreUnavailable)?;
        if r.rows_affected == 1 {
            Ok(())
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    pub async fn set_disabled(&self, username: &str, disabled: bool) -> Result<(), AuthError> {
        let username = canonical_local_username(username)?;
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username))
            .map_err(|_| AuthError::Internal)?;
        b.insert("disabled", DbValue::Int(if disabled { 1 } else { 0 }))
            .map_err(|_| AuthError::Internal)?;
        let r=self.db.execute(&PreparedSql::compile("UPDATE _velran_local_users SET disabled=:disabled,auth_generation=auth_generation+1 WHERE username=:username").map_err(|_|AuthError::Internal)?,&b).await.map_err(|_|AuthError::StoreUnavailable)?;
        if r.rows_affected == 1 {
            Ok(())
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<LocalUserAuth, AuthError> {
        let username = canonical_local_username(username)?;
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username.clone()))
            .map_err(|_| AuthError::Internal)?;
        let shape = RowShape {
            columns: vec![
                ColumnSpec {
                    name: "password_hash".into(),
                    ty: DbScalarType::String,
                },
                ColumnSpec {
                    name: "disabled".into(),
                    ty: DbScalarType::Int,
                },
                ColumnSpec {
                    name: "totp_secret".into(),
                    ty: DbScalarType::Bytes,
                },
                ColumnSpec {
                    name: "auth_generation".into(),
                    ty: DbScalarType::Int,
                },
            ],
        };
        let rows=self.db.fetch_all(&PreparedSql::compile("SELECT password_hash,disabled,COALESCE(totp_secret,X'') AS totp_secret,auth_generation FROM _velran_local_users WHERE username=:username").map_err(|_|AuthError::Internal)?,&b,&shape).await.map_err(|_|AuthError::StoreUnavailable)?;
        let Some(row) = rows.first() else {
            dummy_password_verify(password);
            return Err(AuthError::InvalidCredentials);
        };
        let hash = match row.get("password_hash") {
            Some(DbValue::String(v)) => v,
            _ => return Err(AuthError::StoreUnavailable),
        };
        let disabled = matches!(row.get("disabled"),Some(DbValue::Int(v)) if *v!=0);
        if !verify_password(hash, password) {
            return Err(AuthError::InvalidCredentials);
        }
        if disabled {
            return Err(AuthError::InvalidCredentials);
        }
        let totp_secret = match row.get("totp_secret") {
            Some(DbValue::Bytes(v)) if !v.is_empty() => Some(v.clone()),
            Some(DbValue::Bytes(_)) => None,
            _ => return Err(AuthError::StoreUnavailable),
        };
        let auth_generation = match row.get("auth_generation") {
            Some(DbValue::Int(v)) if *v >= 0 => *v as u64,
            _ => return Err(AuthError::StoreUnavailable),
        };
        let roles = self.roles(&username).await?;
        let memberships = self.memberships(&username).await?;
        Ok(LocalUserAuth {
            username,
            roles,
            memberships,
            totp_secret,
            auth_generation,
        })
    }

    pub async fn roles(&self, username: &str) -> Result<Vec<String>, AuthError> {
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username.into()))
            .map_err(|_| AuthError::Internal)?;
        let shape = RowShape {
            columns: vec![ColumnSpec {
                name: "role".into(),
                ty: DbScalarType::String,
            }],
        };
        let rows = self
            .db
            .fetch_all(
                &PreparedSql::compile(
                    "SELECT role FROM _velran_local_user_roles WHERE username=:username ORDER BY role",
                )
                .map_err(|_| AuthError::Internal)?,
                &b,
                &shape,
            )
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        rows.into_iter()
            .map(|r| match r.get("role") {
                Some(DbValue::String(v)) => Ok(v.clone()),
                _ => Err(AuthError::StoreUnavailable),
            })
            .collect()
    }

    pub async fn set_roles(&self, username: &str, roles: &[String]) -> Result<(), AuthError> {
        let username = canonical_local_username(username)?;
        validate_roles(roles)?;
        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username.clone()))
            .map_err(|_| AuthError::Internal)?;
        tx.execute(
            &PreparedSql::compile("DELETE FROM _velran_local_user_roles WHERE username=:username")
                .map_err(|_| AuthError::Internal)?,
            &b,
        )
        .await
        .map_err(|_| AuthError::StoreUnavailable)?;
        for role in roles {
            let mut rb = BindSet::new();
            rb.insert("username", DbValue::String(username.clone()))
                .map_err(|_| AuthError::Internal)?;
            rb.insert("role", DbValue::String(role.clone()))
                .map_err(|_| AuthError::Internal)?;
            tx.execute(
                &PreparedSql::compile(
                    "INSERT INTO _velran_local_user_roles(username,role) VALUES(:username,:role)",
                )
                .map_err(|_| AuthError::Internal)?,
                &rb,
            )
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        }
        tx.execute(&PreparedSql::compile("UPDATE _velran_local_users SET auth_generation=auth_generation+1 WHERE username=:username").map_err(|_|AuthError::Internal)?,&b).await.map_err(|_|AuthError::StoreUnavailable)?;
        tx.commit().await.map_err(|_| AuthError::StoreUnavailable)
    }

    pub async fn enroll_totp(
        &self,
        username: &str,
        recovery_count: usize,
    ) -> Result<(String, Vec<String>), AuthError> {
        let username = canonical_local_username(username)?;
        if recovery_count == 0 || recovery_count > 32 {
            return Err(AuthError::Internal);
        }
        let mut secret = vec![0u8; 20];
        rand::fill(&mut secret[..]);
        let recovery: (Vec<String>, Vec<String>) = (0..recovery_count)
            .map(|_| {
                let code = format!("{}-{}", random_hex(5), random_hex(5));
                let hash = recovery_hash(&username, &code);
                (code, hash)
            })
            .unzip();
        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username.clone()))
            .map_err(|_| AuthError::Internal)?;
        b.insert("secret", DbValue::Bytes(secret.clone()))
            .map_err(|_| AuthError::Internal)?;
        let r=tx.execute(&PreparedSql::compile("UPDATE _velran_local_users SET totp_secret=:secret,auth_generation=auth_generation+1 WHERE username=:username").map_err(|_|AuthError::Internal)?,&b).await.map_err(|_|AuthError::StoreUnavailable)?;
        if r.rows_affected != 1 {
            return Err(AuthError::InvalidCredentials);
        }
        let mut delete_b = BindSet::new();
        delete_b
            .insert("username", DbValue::String(username.clone()))
            .map_err(|_| AuthError::Internal)?;
        tx.execute(
            &PreparedSql::compile(
                "DELETE FROM _velran_local_recovery_codes WHERE username=:username",
            )
            .map_err(|_| AuthError::Internal)?,
            &delete_b,
        )
        .await
        .map_err(|_| AuthError::StoreUnavailable)?;
        for hash in &recovery.1 {
            let mut rb = BindSet::new();
            rb.insert("username", DbValue::String(username.clone()))
                .map_err(|_| AuthError::Internal)?;
            rb.insert("hash", DbValue::String(hash.clone()))
                .map_err(|_| AuthError::Internal)?;
            tx.execute(&PreparedSql::compile("INSERT INTO _velran_local_recovery_codes(username,code_hash) VALUES(:username,:hash)").map_err(|_|AuthError::Internal)?,&rb).await.map_err(|_|AuthError::StoreUnavailable)?;
        }
        tx.commit().await.map_err(|_| AuthError::StoreUnavailable)?;
        Ok((hex_bytes(&secret), recovery.0))
    }

    pub async fn disable_totp(&self, username: &str) -> Result<(), AuthError> {
        let username = canonical_local_username(username)?;
        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username))
            .map_err(|_| AuthError::Internal)?;
        let r=tx.execute(&PreparedSql::compile("UPDATE _velran_local_users SET totp_secret=NULL,auth_generation=auth_generation+1 WHERE username=:username").map_err(|_|AuthError::Internal)?,&b).await.map_err(|_|AuthError::StoreUnavailable)?;
        if r.rows_affected != 1 {
            return Err(AuthError::InvalidCredentials);
        }
        tx.execute(
            &PreparedSql::compile(
                "DELETE FROM _velran_local_recovery_codes WHERE username=:username",
            )
            .map_err(|_| AuthError::Internal)?,
            &b,
        )
        .await
        .map_err(|_| AuthError::StoreUnavailable)?;
        tx.commit().await.map_err(|_| AuthError::StoreUnavailable)
    }

    pub async fn consume_recovery_code(
        &self,
        username: &str,
        code: &str,
    ) -> Result<bool, AuthError> {
        let username = canonical_local_username(username)?;
        if code.len() > 64 {
            return Ok(false);
        }
        let hash = recovery_hash(&username, code);
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username))
            .map_err(|_| AuthError::Internal)?;
        b.insert("hash", DbValue::String(hash))
            .map_err(|_| AuthError::Internal)?;
        let r=self.db.execute(&PreparedSql::compile("DELETE FROM _velran_local_recovery_codes WHERE username=:username AND code_hash=:hash").map_err(|_|AuthError::Internal)?,&b).await.map_err(|_|AuthError::StoreUnavailable)?;
        Ok(r.rows_affected == 1)
    }
}

fn canonical_local_username(raw: &str) -> Result<String, AuthError> {
    let v = raw.trim();
    if v.is_empty()
        || v.len() > 128
        || !v.is_ascii()
        || !v
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'@'))
    {
        return Err(AuthError::InvalidCredentials);
    }
    Ok(v.to_ascii_lowercase())
}
fn validate_roles(roles: &[String]) -> Result<(), AuthError> {
    if roles.len() > 64 {
        return Err(AuthError::Internal);
    }
    for r in roles {
        if r.is_empty()
            || r.len() > 64
            || !r.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(AuthError::Internal);
        }
    }
    Ok(())
}
fn validate_password(password: &str) -> Result<(), AuthError> {
    if password.len() < 12 || password.len() > 1024 {
        return Err(AuthError::InvalidCredentials);
    }
    Ok(())
}
fn argon2_instance() -> Argon2<'static> {
    Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(19 * 1024, 2, 1, None).expect("valid Argon2id params"),
    )
}
fn hash_password(password: &str) -> Result<String, AuthError> {
    let mut salt = [0u8; 16];
    rand::fill(&mut salt[..]);
    let salt = SaltString::encode_b64(&salt).map_err(|_| AuthError::Internal)?;
    argon2_instance()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AuthError::Internal)
}
fn verify_password(encoded: &str, password: &str) -> bool {
    PasswordHash::new(encoded).ok().is_some_and(|parsed| {
        argon2_instance()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}
fn dummy_password_verify(password: &str) {
    let _ = hash_password(password);
}
fn recovery_hash(username: &str, code: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"velran-recovery-v1\0");
    h.update(username.as_bytes());
    h.update(b"\0");
    h.update(code.as_bytes());
    hex_bytes(&h.finalize())
}
fn hex_bytes(v: &[u8]) -> String {
    let mut out = String::with_capacity(v.len() * 2);
    for b in v {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
    }
    out
}

impl LocalUserStore {
    pub async fn session_generation(&self, username: &str) -> Result<Option<u64>, AuthError> {
        let username = canonical_local_username(username)?;
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username))
            .map_err(|_| AuthError::Internal)?;
        let shape = RowShape {
            columns: vec![
                ColumnSpec {
                    name: "disabled".into(),
                    ty: DbScalarType::Int,
                },
                ColumnSpec {
                    name: "auth_generation".into(),
                    ty: DbScalarType::Int,
                },
            ],
        };
        let rows = self
            .db
            .fetch_all(
                &PreparedSql::compile(
                    "SELECT disabled,auth_generation FROM _velran_local_users WHERE username=:username",
                )
                .map_err(|_| AuthError::Internal)?,
                &b,
                &shape,
            )
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        if matches!(row.get("disabled"),Some(DbValue::Int(v)) if *v!=0) {
            return Ok(None);
        };
        match row.get("auth_generation") {
            Some(DbValue::Int(v)) if *v >= 0 => Ok(Some(*v as u64)),
            _ => Err(AuthError::StoreUnavailable),
        }
    }
}

impl LocalUserStore {
    pub async fn ensure_ready(&self) -> Result<(), AuthError> {
        let shape = RowShape {
            columns: vec![ColumnSpec {
                name: "username".into(),
                ty: DbScalarType::String,
            }],
        };
        self.db
            .fetch_all(
                &PreparedSql::compile(
                    "SELECT username FROM _velran_local_users ORDER BY username LIMIT 1",
                )
                .map_err(|_| AuthError::Internal)?,
                &BindSet::new(),
                &shape,
            )
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        let membership_shape = RowShape {
            columns: vec![ColumnSpec {
                name: "tenant_id".into(),
                ty: DbScalarType::String,
            }],
        };
        self.db
            .fetch_all(
                &PreparedSql::compile(
                    "SELECT tenant_id FROM _velran_local_user_memberships ORDER BY tenant_id LIMIT 1",
                )
                .map_err(|_| AuthError::Internal)?,
                &BindSet::new(),
                &membership_shape,
            )
            .await
            .map(|_| ())
            .map_err(|_| AuthError::StoreUnavailable)
    }
}

#[cfg(test)]
mod local_auth_tests {
    use super::*;

    #[test]
    fn argon2id_password_roundtrip() {
        let h = hash_password("correct horse battery staple").unwrap();
        assert!(h.starts_with("$argon2id$v=19$"));
        assert!(verify_password(&h, "correct horse battery staple"));
        assert!(!verify_password(&h, "wrong password"));
    }

    #[tokio::test]
    async fn local_store_login_disable_totp_recovery() {
        let path = std::env::temp_dir().join(format!("velran-local-auth-{}.db", random_hex(8)));
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let store = LocalUserStore::connect_sqlite(&url).await.unwrap();
        store.initialize().await.unwrap();
        store
            .create_user("Alice", "a very long local password", &["Editor".into()])
            .await
            .unwrap();
        let user = store
            .authenticate("alice", "a very long local password")
            .await
            .unwrap();
        assert_eq!(user.username, "alice");
        assert_eq!(user.roles, vec!["Editor"]);
        let g1 = user.auth_generation;
        store
            .set_roles("alice", &["Publisher".into()])
            .await
            .unwrap();
        let g2 = store.session_generation("alice").await.unwrap().unwrap();
        assert!(g2 > g1);
        let tenant = TenantId::parse("acme").unwrap();
        store
            .set_memberships("alice", std::slice::from_ref(&tenant))
            .await
            .unwrap();
        let user = store
            .authenticate("alice", "a very long local password")
            .await
            .unwrap();
        assert_eq!(user.memberships, vec![tenant]);
        assert!(user.auth_generation > g2);
        let (_secret, codes) = store.enroll_totp("alice", 4).await.unwrap();
        // enroll_totp uses statement-specific BindSets: the strict data layer rejects extra binds.
        assert_eq!(codes.len(), 4);
        assert!(
            store
                .consume_recovery_code("alice", &codes[0])
                .await
                .unwrap()
        );
        assert!(
            !store
                .consume_recovery_code("alice", &codes[0])
                .await
                .unwrap()
        );
        store.set_disabled("alice", true).await.unwrap();
        assert_eq!(
            store
                .authenticate("alice", "a very long local password")
                .await
                .unwrap_err(),
            AuthError::InvalidCredentials
        );
        let _ = std::fs::remove_file(path);
    }
}
