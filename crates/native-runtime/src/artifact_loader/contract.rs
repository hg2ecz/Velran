use executable_ir::shard_planner::VerifiedShard;

use super::{LoadError, unix::Library};

pub(super) fn verify(library: &Library, shard: &VerifiedShard) -> Result<(), LoadError> {
    let actual_abi = library.abi_version();
    if actual_abi != runtime_abi::RUNTIME_ABI_VERSION {
        return Err(LoadError::AbiMismatch {
            expected: runtime_abi::RUNTIME_ABI_VERSION,
            actual: actual_abi,
        });
    }
    let expected_contract = native_build::artifact_format::contract_fingerprint(shard);
    let actual_contract = library.contract_fingerprint();
    if actual_contract != expected_contract {
        return Err(LoadError::ContractMismatch {
            expected: expected_contract,
            actual: actual_contract,
        });
    }
    Ok(())
}
