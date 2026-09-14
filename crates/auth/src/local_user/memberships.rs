use super::{LocalUserStore, canonical_local_username};
use crate::{AuthError, TenantId, validate_memberships};
use data::{BindSet, ColumnSpec, DbScalarType, DbValue, PreparedSql, RowShape};

impl LocalUserStore {
    pub async fn memberships(&self, username: &str) -> Result<Vec<TenantId>, AuthError> {
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username.into()))
            .map_err(|_| AuthError::Internal)?;
        let shape = RowShape {
            columns: vec![ColumnSpec {
                name: "tenant_id".into(),
                ty: DbScalarType::String,
            }],
        };
        let rows = self
            .db
            .fetch_all(
                &PreparedSql::compile(
                    "SELECT tenant_id FROM _velran_local_user_memberships WHERE username=:username ORDER BY tenant_id",
                )
                .map_err(|_| AuthError::Internal)?,
                &b,
                &shape,
            )
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        let values: Result<Vec<_>, _> = rows
            .into_iter()
            .map(|row| match row.get("tenant_id") {
                Some(DbValue::String(value)) => TenantId::parse(value),
                _ => Err(AuthError::StoreUnavailable),
            })
            .collect();
        let values = values?;
        validate_memberships(&values)?;
        Ok(values)
    }

    pub async fn set_memberships(
        &self,
        username: &str,
        memberships: &[TenantId],
    ) -> Result<(), AuthError> {
        let username = canonical_local_username(username)?;
        validate_memberships(memberships)?;
        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        let mut b = BindSet::new();
        b.insert("username", DbValue::String(username.clone()))
            .map_err(|_| AuthError::Internal)?;
        tx.execute(
            &PreparedSql::compile(
                "DELETE FROM _velran_local_user_memberships WHERE username=:username",
            )
            .map_err(|_| AuthError::Internal)?,
            &b,
        )
        .await
        .map_err(|_| AuthError::StoreUnavailable)?;
        for tenant in memberships {
            let mut mb = BindSet::new();
            mb.insert("username", DbValue::String(username.clone()))
                .map_err(|_| AuthError::Internal)?;
            mb.insert("tenant", DbValue::String(tenant.as_str().into()))
                .map_err(|_| AuthError::Internal)?;
            tx.execute(
                &PreparedSql::compile(
                    "INSERT INTO _velran_local_user_memberships(username,tenant_id) VALUES(:username,:tenant)",
                )
                .map_err(|_| AuthError::Internal)?,
                &mb,
            )
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        }
        tx.execute(
            &PreparedSql::compile(
                "UPDATE _velran_local_users SET auth_generation=auth_generation+1 WHERE username=:username",
            )
            .map_err(|_| AuthError::Internal)?,
            &b,
        )
        .await
        .map_err(|_| AuthError::StoreUnavailable)?;
        tx.commit().await.map_err(|_| AuthError::StoreUnavailable)
    }
}
