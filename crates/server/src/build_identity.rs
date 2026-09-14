use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;

const SERVER_BUILD_CONTRACT_VERSION: &str = "server-build-contract-2-native-cache-fastpath";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BuildIdentityError {
    InvalidExpectedShape,
    Mismatch { expected: String, actual: String },
}

impl fmt::Display for BuildIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExpectedShape => f.write_str(
                "server.expected_build_id must be exactly 64 lowercase hexadecimal characters",
            ),
            Self::Mismatch { expected, actual } => write!(
                f,
                "server.expected_build_id mismatch: configured {expected}, running binary {actual}",
            ),
        }
    }
}

impl Error for BuildIdentityError {}

pub(super) fn current() -> String {
    let mut hash = Sha256::new();
    hash.update(b"velran-server-build-id-v1");
    hash.update(env!("CARGO_PKG_VERSION").as_bytes());
    hash.update([0]);
    hash.update(SERVER_BUILD_CONTRACT_VERSION.as_bytes());
    hash.update([0]);
    hash.update(runtime_abi::RUNTIME_ABI_VERSION.to_le_bytes());
    hash.update(language_core::LANGUAGE_VERSION.as_bytes());
    hash.update([0]);
    hash.update(language_core::SECURITY_POLICY_VERSION.as_bytes());
    hash.update([0]);
    hash.update(compiler::codegen::CODEGEN_VERSION.as_bytes());
    format!("{:x}", hash.finalize())
}

pub(super) fn validate_expected(expected: &str) -> Result<(), BuildIdentityError> {
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(BuildIdentityError::InvalidExpectedShape);
    }
    let actual = current();
    if expected == actual {
        Ok(())
    } else {
        Err(BuildIdentityError::Mismatch {
            expected: expected.to_owned(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_id_is_stable_hex_sha256() {
        let id = current();
        assert_eq!(id.len(), 64);
        assert!(
            id.bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
    }

    #[test]
    fn expected_build_id_is_fail_closed() {
        assert!(validate_expected(&current()).is_ok());
        assert!(matches!(
            validate_expected("not-a-build-id"),
            Err(BuildIdentityError::InvalidExpectedShape)
        ));
        assert!(matches!(
            validate_expected(&"0".repeat(64)),
            Err(BuildIdentityError::Mismatch { .. })
        ));
    }
}
