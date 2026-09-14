use executable_ir::shard_planner::VerifiedShard;
use sha2::{Digest, Sha256};

pub(crate) fn content_key(kind: &str, payload: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"velran-artifact");
    hash.update([0]);
    hash.update(kind.as_bytes());
    hash.update([0]);
    hash.update(env!("CARGO_PKG_VERSION").as_bytes());
    hash.update([0]);
    hash.update(language_core::LANGUAGE_VERSION.as_bytes());
    hash.update([0]);
    hash.update(language_core::IR_VERSION.to_le_bytes());
    hash.update([0]);
    hash.update(language_core::SECURITY_POLICY_VERSION.as_bytes());
    hash.update([0]);
    hash.update(payload);
    format!("{:x}", hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_kind_is_domain_separated() {
        assert_ne!(
            content_key("source-mods", b"x"),
            content_key("typed-ir", b"x")
        );
    }
}

pub fn runtime_contract_fingerprint(shard: &VerifiedShard) -> u64 {
    let mut hash = Sha256::new();
    hash.update(b"velran-runtime-contract-v1");
    hash.update(runtime_abi::RUNTIME_ABI_VERSION.to_le_bytes());
    hash.update(shard.language_version().as_bytes());
    hash.update([0]);
    hash.update(shard.security_policy_version().as_bytes());
    hash.update([0]);
    hash.update(shard.executable_ir_version().to_le_bytes());
    hash.update(crate::codegen::CODEGEN_VERSION.as_bytes());
    let digest = hash.finalize();
    u64::from_le_bytes(digest[..8].try_into().expect("sha256 prefix length"))
}
