use crate::AuthRuntime;
use sha2::{Digest, Sha256};

impl AuthRuntime {
    pub(super) fn mapped_claims_generation(&self, principal: &str) -> u64 {
        let mut roles = self.roles.get(principal).cloned().unwrap_or_default();
        roles.sort();
        roles.dedup();
        let mut memberships: Vec<_> = self
            .memberships
            .get(principal)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|tenant| tenant.as_str().to_string())
            .collect();
        memberships.sort();
        memberships.dedup();

        let mut digest = Sha256::new();
        digest.update(b"velran-auth-claims-v1\0");
        for role in roles {
            digest.update(b"role\0");
            digest.update(role.as_bytes());
            digest.update([0]);
        }
        for tenant in memberships {
            digest.update(b"tenant\0");
            digest.update(tenant.as_bytes());
            digest.update([0]);
        }
        let bytes = digest.finalize();
        u64::from_be_bytes(bytes[..8].try_into().expect("SHA-256 prefix"))
    }
}
