#![forbid(unsafe_code)]

mod host;
mod input;
mod result;

pub use host::{
    HOST_OP_OUTBOUND_GET_STATUS, HOST_OP_OUTBOUND_POST_JSON_STATUS, HOST_STATUS_BAD_REQUEST,
    HOST_STATUS_FAILED, HOST_STATUS_OK, HOST_STATUS_UNAVAILABLE, HOST_VALUE_INT, HOST_VALUE_NONE,
    MAX_HOST_REQUEST_BYTES, MAX_HOST_RESPONSE_BYTES, VelranHostApi, VelranHostCall,
    VelranHostResult,
};
pub use input::{
    INPUT_BOOL, INPUT_DOMAIN_BOOL, INPUT_DOMAIN_INT, INPUT_DOMAIN_STRING, INPUT_EMAIL, INPUT_IMAGE,
    INPUT_INT, INPUT_SLUG, INPUT_STRING, INPUT_UPLOAD, INPUT_URL, MAX_INPUT_FIELDS, RequestValue,
    VelranInputValue,
};
pub use result::VelranResult;

pub const RUNTIME_ABI_VERSION: u32 = 10;
pub const STATUS_OK: u32 = 0;
pub const STATUS_INTERNAL: u32 = 1;
pub const STATUS_BUDGET_EXCEEDED: u32 = 2;
pub const STATUS_MEMORY_EXCEEDED: u32 = 3;
pub const STATUS_BAD_REQUEST: u32 = 4;
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 5;
pub const STATUS_UNSUPPORTED: u32 = 0xFFFF_FF01;
pub const VALUE_NONE: u32 = 0;
pub const VALUE_INT: u32 = 1;
pub const VALUE_BOOL: u32 = 2;
pub const VALUE_HTML: u32 = 3;
pub const VALUE_TYPED_JSON: u32 = 4;
pub const MAX_OUTPUT_BYTES: usize = 1_048_576;
pub const SYMBOL_ABI_VERSION: &str = "velran_module_abi_version";
pub const SYMBOL_INVOKE: &str = "velran_invoke";

#[cfg(test)]
mod result_tests;
