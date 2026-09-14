use super::{LoadError, unix_symbols};
use runtime_abi::{VelranHostApi, VelranInputValue, VelranResult};
use std::ffi::c_void;

const ABI_SYMBOL: &[u8] = b"velran_module_abi_version\0";
const CONTRACT_SYMBOL: &[u8] = b"velran_module_contract_fingerprint\0";
const INVOKE_SYMBOL: &[u8] = b"velran_invoke\0";

pub(super) type AbiVersionFn = unsafe extern "C" fn() -> u32;
pub(super) type ContractFingerprintFn = unsafe extern "C" fn() -> u64;
pub(super) type InvokeFn = unsafe extern "C" fn(
    u32,
    *const VelranHostApi,
    *const VelranInputValue,
    u32,
    *mut u8,
    u64,
    u64,
    u64,
) -> VelranResult;

pub(super) fn resolve(
    handle: *mut c_void,
) -> Result<(AbiVersionFn, ContractFingerprintFn, InvokeFn), LoadError> {
    let abi = unix_symbols::resolve(handle, ABI_SYMBOL, LoadError::MissingAbiSymbol)?;
    let contract =
        unix_symbols::resolve(handle, CONTRACT_SYMBOL, LoadError::MissingContractSymbol)?;
    let invoke = unix_symbols::resolve(handle, INVOKE_SYMBOL, LoadError::MissingInvokeSymbol)?;
    Ok(unsafe {
        (
            std::mem::transmute::<*mut c_void, AbiVersionFn>(abi),
            std::mem::transmute::<*mut c_void, ContractFingerprintFn>(contract),
            std::mem::transmute::<*mut c_void, InvokeFn>(invoke),
        )
    })
}
