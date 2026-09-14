use runtime_abi::{MAX_INPUT_FIELDS, MAX_OUTPUT_BYTES, RUNTIME_ABI_VERSION};
use std::sync::OnceLock;

pub(crate) fn source() -> &'static str {
    static SOURCE: OnceLock<String> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let validation = super::abi_input_validation::SOURCE;
        let host = super::abi_host::SOURCE;
        format!(
        r#"use std::ffi::c_void;
const VELRAN_INPUT_INT: u32 = 1;
const VELRAN_INPUT_BOOL: u32 = 2;
const VELRAN_INPUT_STRING: u32 = 3;
const VELRAN_INPUT_EMAIL: u32 = 4;
const VELRAN_INPUT_URL: u32 = 5;
const VELRAN_INPUT_SLUG: u32 = 6;
const VELRAN_INPUT_DOMAIN_INT: u32 = 7;
const VELRAN_INPUT_DOMAIN_BOOL: u32 = 8;
const VELRAN_INPUT_DOMAIN_STRING: u32 = 9;
const VELRAN_INPUT_UPLOAD: u32 = 10;
const VELRAN_INPUT_IMAGE: u32 = 11;
const VELRAN_MAX_INPUT_FIELDS: usize = {MAX_INPUT_FIELDS};
const VELRAN_MAX_INPUT_STRING_BYTES: usize = 1_048_576;
const VELRAN_MAX_OUTPUT_BYTES: usize = {MAX_OUTPUT_BYTES};
const VELRAN_STATUS_OUTPUT_TOO_SMALL: u32 = 5;
const VELRAN_VALUE_HTML: u32 = 3;
const VELRAN_STATUS_BAD_REQUEST: u32 = 4;
const VELRAN_VALUE_NONE: u32 = 0;
#[repr(C)]
pub struct VelranResult {{ pub status: u32, pub value_tag: u32, pub payload: u64, pub fuel_used: u64, pub allocated_bytes: u64, pub output_len: u64 }}
#[repr(C)]
pub struct VelranInputValue {{ pub tag: u32, pub aux: u32, pub payload: u64, pub data: *const u8, pub len: u64 }}
{host}
#[unsafe(no_mangle)] pub extern "C" fn velran_module_abi_version() -> u32 {{ {RUNTIME_ABI_VERSION} }}
#[unsafe(no_mangle)] pub extern "C" fn velran_module_contract_fingerprint() -> u64 {{ implementation::VELRAN_RUNTIME_CONTRACT_FINGERPRINT }}
fn bad_request() -> VelranResult {{ VelranResult {{ status: VELRAN_STATUS_BAD_REQUEST, value_tag: VELRAN_VALUE_NONE, payload: 0, fuel_used: 0, allocated_bytes: 0, output_len: 0 }} }}
{validation}
#[unsafe(no_mangle)]
pub extern "C" fn velran_invoke(handler_id: u32, host: *const VelranHostApi, inputs: *const VelranInputValue, input_len: u32, output: *mut u8, output_cap: u64, instruction_budget: u64, allocation_budget: u64) -> VelranResult {{
    if host.is_null() || input_len > VELRAN_MAX_INPUT_FIELDS as u32 || (input_len != 0 && inputs.is_null()) {{ return bad_request(); }}
    let host = unsafe {{ &*host }};
    if host.abi_version != {RUNTIME_ABI_VERSION} {{ return bad_request(); }}
    let Ok(output_cap) = usize::try_from(output_cap) else {{ return bad_request(); }};
    if output_cap > VELRAN_MAX_OUTPUT_BYTES || (output_cap != 0 && output.is_null()) {{ return bad_request(); }}
    let raw_inputs = if input_len == 0 {{ &[][..] }} else {{ unsafe {{ std::slice::from_raw_parts(inputs, input_len as usize) }} }};
    let mut decoded = [implementation::InputValue::Int(0); VELRAN_MAX_INPUT_FIELDS];
    for (index, raw) in raw_inputs.iter().enumerate() {{
        decoded[index] = match raw.tag {{
            VELRAN_INPUT_INT if raw.aux == 0 && raw.data.is_null() && raw.len == 0 => implementation::InputValue::Int(raw.payload as i64),
            VELRAN_INPUT_BOOL if raw.aux == 0 && raw.data.is_null() && raw.len == 0 && raw.payload <= 1 => implementation::InputValue::Bool(raw.payload != 0),
            VELRAN_INPUT_STRING if raw.aux == 0 => match text(raw, VELRAN_MAX_INPUT_STRING_BYTES) {{ Some(v) => implementation::InputValue::String(v), None => return bad_request() }},
            VELRAN_INPUT_EMAIL if raw.aux == 0 => match text(raw, 254) {{ Some(v) if canonical_email(v) => implementation::InputValue::Email(v), _ => return bad_request() }},
            VELRAN_INPUT_URL if raw.aux == 0 => match text(raw, 2048) {{ Some(v) if safe_normalized_url(v) => implementation::InputValue::Url(v), _ => return bad_request() }},
            VELRAN_INPUT_SLUG if raw.aux == 0 => match text(raw, 160) {{ Some(v) if canonical_slug(v) => implementation::InputValue::Slug(v), _ => return bad_request() }},
            VELRAN_INPUT_DOMAIN_INT if raw.aux <= u16::MAX as u32 && raw.data.is_null() && raw.len == 0 => implementation::InputValue::DomainInt(raw.aux as u16, raw.payload as i64),
            VELRAN_INPUT_DOMAIN_BOOL if raw.aux <= u16::MAX as u32 && raw.data.is_null() && raw.len == 0 && raw.payload <= 1 => implementation::InputValue::DomainBool(raw.aux as u16, raw.payload != 0),
            VELRAN_INPUT_DOMAIN_STRING if raw.aux <= u16::MAX as u32 => match text(raw, VELRAN_MAX_INPUT_STRING_BYTES) {{ Some(v) => implementation::InputValue::DomainString(raw.aux as u16, v), None => return bad_request() }},
            VELRAN_INPUT_UPLOAD if raw.aux == 0 && raw.payload == 0 => match bytes(raw, VELRAN_MAX_INPUT_STRING_BYTES) {{ Some(v) => implementation::InputValue::Upload(v), None => return bad_request() }},
            VELRAN_INPUT_IMAGE if raw.aux == 0 && raw.payload == 0 => match bytes(raw, VELRAN_MAX_INPUT_STRING_BYTES) {{ Some(v) => implementation::InputValue::Image(v), None => return bad_request() }},
            _ => return bad_request(),
        }};
    }}
    let output = if output_cap == 0 {{ &mut [][..] }} else {{ unsafe {{ std::slice::from_raw_parts_mut(output, output_cap) }} }};
    let result = implementation::velran_invoke_safe(handler_id, host, &decoded[..input_len as usize], output, instruction_budget, allocation_budget);
    let (status, value_tag, payload, fuel_used, allocated_bytes, output_len) = result;
    if status == VELRAN_STATUS_OUTPUT_TOO_SMALL && value_tag != VELRAN_VALUE_HTML {{ return bad_request(); }}
    VelranResult {{ status, value_tag, payload, fuel_used, allocated_bytes, output_len }}
}}
"#
        )
    }).as_str()
}
