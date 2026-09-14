use executable_ir::shard_planner::VerifiedShard;
use sha2::{Digest, Sha256};

use super::IntegrityError;

pub(super) fn generated_source_sha256(shard: &VerifiedShard) -> Result<String, IntegrityError> {
    let generated =
        compiler::codegen::generate(shard).map_err(|_| IntegrityError::ShardContentMismatch)?;
    Ok(hex_sha256(generated.source.as_bytes()))
}

pub(super) fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_encoding_is_stable() {
        assert_eq!(
            hex_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn constant_time_compare_rejects_difference() {
        assert!(constant_time_eq(b"abcd", b"abcd"));
        assert!(!constant_time_eq(b"abcd", b"abce"));
    }
}
