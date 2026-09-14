use runtime::OutboundRuntime;
use runtime_abi::{
    HOST_OP_OUTBOUND_GET_STATUS, HOST_OP_OUTBOUND_POST_JSON_STATUS, HOST_STATUS_BAD_REQUEST,
    HOST_STATUS_FAILED, HOST_STATUS_UNAVAILABLE, RUNTIME_ABI_VERSION, VelranHostApi,
    VelranHostResult,
};
use std::ffi::c_void;

pub(super) struct NativeHostBridge<'a> {
    outbound: Option<&'a dyn OutboundRuntime>,
    runtime: tokio::runtime::Handle,
    remaining_external_io_bytes: u64,
}

impl<'a> NativeHostBridge<'a> {
    pub(super) fn new(outbound: Option<&'a dyn OutboundRuntime>, external_io_budget: u64) -> Self {
        Self {
            outbound,
            runtime: tokio::runtime::Handle::current(),
            remaining_external_io_bytes: external_io_budget,
        }
    }

    pub(super) fn api(&mut self) -> VelranHostApi {
        VelranHostApi {
            abi_version: RUNTIME_ABI_VERSION,
            context: (self as *mut Self).cast::<c_void>(),
            call: Some(host_call),
        }
    }
}

extern "C" fn host_call(
    context: *mut c_void,
    operation: u32,
    request: *const u8,
    request_len: u64,
    _response: *mut u8,
    response_cap: u64,
) -> VelranHostResult {
    if context.is_null() || response_cap != 0 {
        return VelranHostResult::failed(HOST_STATUS_BAD_REQUEST);
    }
    let Ok(request_len) = usize::try_from(request_len) else {
        return VelranHostResult::failed(HOST_STATUS_BAD_REQUEST);
    };
    if request_len > runtime_abi::MAX_HOST_REQUEST_BYTES || (request_len != 0 && request.is_null())
    {
        return VelranHostResult::failed(HOST_STATUS_BAD_REQUEST);
    }
    let request = if request_len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(request, request_len) }
    };
    let bridge = unsafe { &mut *(context.cast::<NativeHostBridge<'_>>()) };
    match operation {
        HOST_OP_OUTBOUND_GET_STATUS | HOST_OP_OUTBOUND_POST_JSON_STATUS => {
            outbound_call(bridge, operation, request)
        }
        _ => VelranHostResult::failed(HOST_STATUS_BAD_REQUEST),
    }
}

fn outbound_call(
    bridge: &mut NativeHostBridge<'_>,
    operation: u32,
    request: &[u8],
) -> VelranHostResult {
    let Ok(charge) = u64::try_from(request.len()) else {
        return VelranHostResult::failed(HOST_STATUS_BAD_REQUEST);
    };
    let Some(remaining) = bridge.remaining_external_io_bytes.checked_sub(charge) else {
        return VelranHostResult::failed(HOST_STATUS_FAILED);
    };
    bridge.remaining_external_io_bytes = remaining;
    let Some(outbound) = bridge.outbound else {
        return VelranHostResult::failed(HOST_STATUS_UNAVAILABLE);
    };
    let Some((method, target, path, body)) = decode_outbound_request(request) else {
        return VelranHostResult::failed(HOST_STATUS_BAD_REQUEST);
    };
    if (operation == HOST_OP_OUTBOUND_GET_STATUS && method != 0)
        || (operation == HOST_OP_OUTBOUND_POST_JSON_STATUS && method != 1)
    {
        return VelranHostResult::failed(HOST_STATUS_BAD_REQUEST);
    }
    let result = tokio::task::block_in_place(|| {
        if method == 0 {
            bridge.runtime.block_on(outbound.get_status(target, path))
        } else {
            bridge
                .runtime
                .block_on(outbound.post_json_status(target, path, body))
        }
    });
    match result {
        Ok(outcome) => VelranHostResult::int(i64::from(outcome.status)),
        Err(()) => VelranHostResult::failed(HOST_STATUS_FAILED),
    }
}

fn decode_outbound_request(raw: &[u8]) -> Option<(u8, &str, &str, &[u8])> {
    let mut at = 0usize;
    let method = *raw.get(at)?;
    if method > 1 {
        return None;
    }
    at += 1;
    let target = read_text(raw, &mut at, 512)?;
    let path = read_text(raw, &mut at, 2048)?;
    let body_len = usize::try_from(read_u32(raw, &mut at)?).ok()?;
    if body_len > runtime_abi::MAX_HOST_REQUEST_BYTES {
        return None;
    }
    let end = at.checked_add(body_len)?;
    let body = raw.get(at..end)?;
    if end != raw.len() {
        return None;
    }
    Some((method, target, path, body))
}

fn read_text<'a>(raw: &'a [u8], at: &mut usize, max: usize) -> Option<&'a str> {
    let len = usize::try_from(read_u32(raw, at)?).ok()?;
    if len > max {
        return None;
    }
    let end = at.checked_add(len)?;
    let value = std::str::from_utf8(raw.get(*at..end)?).ok()?;
    *at = end;
    Some(value)
}

fn read_u32(raw: &[u8], at: &mut usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    let bytes: [u8; 4] = raw.get(*at..end)?.try_into().ok()?;
    *at = end;
    Some(u32::from_le_bytes(bytes))
}
