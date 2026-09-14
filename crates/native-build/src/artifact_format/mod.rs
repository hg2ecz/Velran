#![forbid(unsafe_code)]
use compiler::codegen::CODEGEN_VERSION;
use executable_ir::shard_planner::VerifiedShard;
use runtime_abi::RUNTIME_ABI_VERSION;
use std::fs;
use std::path::Path;

mod codec;
mod error;
mod integrity;

pub use error::{IntegrityError, ManifestDecodeError};
use integrity::{constant_time_eq, generated_source_sha256, hex_sha256};
pub const ARTIFACT_MANIFEST_VERSION: u16 = 4;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactManifest {
    pub manifest_version: u16,
    pub shard_id: String,
    pub crate_name: String,
    pub artifact_sha256: String,
    pub artifact_size: u64,
    pub generated_source_sha256: String,
    pub interface_sha256: String,
    pub implementation_sha256: String,
    pub runtime_abi_version: u32,
    pub language_version: String,
    pub security_policy_version: String,
    pub executable_ir_version: u16,
    pub codegen_version: String,
    pub contract_fingerprint: u64,
}

impl ArtifactManifest {
    pub fn from_artifact(
        shard: &VerifiedShard,
        crate_name: String,
        path: &Path,
    ) -> Result<Self, IntegrityError> {
        let bytes = fs::read(path).map_err(|error| IntegrityError::Io(error.kind()))?;
        Ok(Self {
            manifest_version: ARTIFACT_MANIFEST_VERSION,
            shard_id: shard.id().as_str().to_owned(),
            crate_name,
            artifact_sha256: hex_sha256(&bytes),
            artifact_size: bytes.len() as u64,
            generated_source_sha256: generated_source_sha256(shard)?,
            interface_sha256: shard.interface_sha256().to_owned(),
            implementation_sha256: shard.implementation_sha256().to_owned(),
            runtime_abi_version: RUNTIME_ABI_VERSION,
            language_version: shard.language_version().to_owned(),
            security_policy_version: shard.security_policy_version().to_owned(),
            executable_ir_version: shard.executable_ir_version(),
            codegen_version: CODEGEN_VERSION.to_owned(),
            contract_fingerprint: contract_fingerprint(shard),
        })
    }

    pub fn verify_for(&self, shard: &VerifiedShard, path: &Path) -> Result<(), IntegrityError> {
        if self.manifest_version != ARTIFACT_MANIFEST_VERSION {
            return Err(IntegrityError::ManifestVersionMismatch {
                expected: ARTIFACT_MANIFEST_VERSION,
                actual: self.manifest_version,
            });
        }
        if self.shard_id != shard.id().as_str() {
            return Err(IntegrityError::ShardMismatch);
        }
        if !constant_time_eq(
            self.generated_source_sha256.as_bytes(),
            generated_source_sha256(shard)?.as_bytes(),
        ) {
            return Err(IntegrityError::ShardContentMismatch);
        }
        if !constant_time_eq(
            self.interface_sha256.as_bytes(),
            shard.interface_sha256().as_bytes(),
        ) {
            return Err(IntegrityError::ShardContentMismatch);
        }
        if !constant_time_eq(
            self.implementation_sha256.as_bytes(),
            shard.implementation_sha256().as_bytes(),
        ) {
            return Err(IntegrityError::ShardContentMismatch);
        }
        if self.runtime_abi_version != RUNTIME_ABI_VERSION {
            return Err(IntegrityError::AbiMismatch {
                expected: RUNTIME_ABI_VERSION,
                actual: self.runtime_abi_version,
            });
        }
        if self.language_version != shard.language_version() {
            return Err(IntegrityError::LanguageVersionMismatch);
        }
        if self.security_policy_version != shard.security_policy_version() {
            return Err(IntegrityError::SecurityPolicyVersionMismatch);
        }
        if self.executable_ir_version != shard.executable_ir_version() {
            return Err(IntegrityError::ExecutableIrVersionMismatch);
        }
        if self.codegen_version != CODEGEN_VERSION {
            return Err(IntegrityError::CodegenVersionMismatch);
        }
        if self.contract_fingerprint != contract_fingerprint(shard) {
            return Err(IntegrityError::ContractFingerprintMismatch);
        }

        let bytes = fs::read(path).map_err(|error| IntegrityError::Io(error.kind()))?;
        let actual_size = bytes.len() as u64;
        if actual_size != self.artifact_size {
            return Err(IntegrityError::SizeMismatch {
                expected: self.artifact_size,
                actual: actual_size,
            });
        }
        if !constant_time_eq(
            self.artifact_sha256.as_bytes(),
            hex_sha256(&bytes).as_bytes(),
        ) {
            return Err(IntegrityError::HashMismatch);
        }
        Ok(())
    }

    pub fn decode(text: &str) -> Result<Self, ManifestDecodeError> {
        codec::decode(text)
    }

    pub fn encode(&self) -> String {
        codec::encode(self)
    }
}

pub fn contract_fingerprint(shard: &VerifiedShard) -> u64 {
    compiler::runtime_contract_fingerprint(shard)
}
