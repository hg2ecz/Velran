pub(crate) const SOURCE: &str = r#"
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VelranHostResult { pub status: u32, pub value_tag: u32, pub payload: u64, pub output_len: u64 }
pub type VelranHostCall = extern "C" fn(*mut c_void, u32, *const u8, u64, *mut u8, u64) -> VelranHostResult;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VelranHostApi { pub abi_version: u32, pub context: *mut c_void, pub call: Option<VelranHostCall> }
"#;
