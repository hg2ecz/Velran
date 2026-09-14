use std::ffi::c_void;

pub const HOST_OP_OUTBOUND_GET_STATUS: u32 = 1;
pub const HOST_OP_OUTBOUND_POST_JSON_STATUS: u32 = 2;
pub const HOST_STATUS_OK: u32 = 0;
pub const HOST_STATUS_BAD_REQUEST: u32 = 1;
pub const HOST_STATUS_UNAVAILABLE: u32 = 2;
pub const HOST_STATUS_FAILED: u32 = 3;
pub const HOST_VALUE_NONE: u32 = 0;
pub const HOST_VALUE_INT: u32 = 1;
pub const MAX_HOST_REQUEST_BYTES: usize = 1_048_576;
pub const MAX_HOST_RESPONSE_BYTES: usize = 1_048_576;

pub type VelranHostCall = extern "C" fn(
    context: *mut c_void,
    operation: u32,
    request: *const u8,
    request_len: u64,
    response: *mut u8,
    response_cap: u64,
) -> VelranHostResult;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VelranHostApi {
    pub abi_version: u32,
    pub context: *mut c_void,
    pub call: Option<VelranHostCall>,
}

impl VelranHostApi {
    pub const UNAVAILABLE: Self = Self {
        abi_version: crate::RUNTIME_ABI_VERSION,
        context: std::ptr::null_mut(),
        call: None,
    };
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VelranHostResult {
    pub status: u32,
    pub value_tag: u32,
    pub payload: u64,
    pub output_len: u64,
}

impl VelranHostResult {
    pub const fn failed(status: u32) -> Self {
        Self {
            status,
            value_tag: HOST_VALUE_NONE,
            payload: 0,
            output_len: 0,
        }
    }
    pub const fn int(value: i64) -> Self {
        Self {
            status: HOST_STATUS_OK,
            value_tag: HOST_VALUE_INT,
            payload: value as u64,
            output_len: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_abi_structs_are_fixed_layout() {
        assert_eq!(std::mem::size_of::<VelranHostResult>(), 24);
        assert!(std::mem::size_of::<VelranHostApi>() >= 24);
    }
}
