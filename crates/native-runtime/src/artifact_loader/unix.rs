use super::{
    LoadError,
    unix_abi::{self, AbiVersionFn, ContractFingerprintFn, InvokeFn},
    unix_symbols,
};
use runtime_abi::{VelranHostApi, VelranInputValue, VelranResult};
use std::ffi::c_void;
use std::path::Path;

pub(super) struct Library {
    handle: *mut c_void,
    abi_version: AbiVersionFn,
    contract_fingerprint: ContractFingerprintFn,
    invoke: InvokeFn,
}

unsafe impl Send for Library {}
unsafe impl Sync for Library {}

impl Library {
    pub(super) fn open(path: &Path) -> Result<Self, LoadError> {
        let handle = unix_symbols::open(path)?;
        let (abi_version, contract_fingerprint, invoke) = unix_abi::resolve(handle)?;
        Ok(Self {
            handle,
            abi_version,
            contract_fingerprint,
            invoke,
        })
    }

    pub(super) fn abi_version(&self) -> u32 {
        unsafe { (self.abi_version)() }
    }
    pub(super) fn contract_fingerprint(&self) -> u64 {
        unsafe { (self.contract_fingerprint)() }
    }

    pub(super) fn invoke(
        &self,
        handler_id: u32,
        host: &VelranHostApi,
        inputs: &[VelranInputValue],
        output: &mut [u8],
        instruction_budget: u64,
        allocation_budget: u64,
    ) -> VelranResult {
        unsafe {
            (self.invoke)(
                handler_id,
                host as *const VelranHostApi,
                inputs.as_ptr(),
                inputs.len() as u32,
                output.as_mut_ptr(),
                output.len() as u64,
                instruction_budget,
                allocation_budget,
            )
        }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        unix_symbols::close(self.handle);
    }
}
