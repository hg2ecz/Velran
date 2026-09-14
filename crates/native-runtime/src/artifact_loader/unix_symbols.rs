use super::LoadError;
use std::ffi::{CString, c_char, c_int, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const RTLD_NOW: c_int = 2;
const RTLD_LOCAL: c_int = 0;

#[cfg_attr(target_os = "linux", link(name = "dl"))]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
}

pub(super) fn open(path: &Path) -> Result<*mut c_void, LoadError> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| LoadError::DynamicLoad)?;
    let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
    if handle.is_null() {
        Err(LoadError::DynamicLoad)
    } else {
        Ok(handle)
    }
}

pub(super) fn resolve(
    handle: *mut c_void,
    symbol: &'static [u8],
    error: LoadError,
) -> Result<*mut c_void, LoadError> {
    let found = unsafe { dlsym(handle, symbol.as_ptr().cast()) };
    if found.is_null() {
        close(handle);
        Err(error)
    } else {
        Ok(found)
    }
}

pub(super) fn close(handle: *mut c_void) {
    if !handle.is_null() {
        let _ = unsafe { dlclose(handle) };
    }
}
