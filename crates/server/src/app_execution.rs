use data::Database;
use language_core::{AppError, Html, HttpMethod, Program, Route, ServerConfig, Value};
use observability::Metrics;
use runtime::{AppResponse, NativeRequestValue, OutboundRuntime, ResourceProfiles};
use runtime_abi::{
    MAX_INPUT_FIELDS, MAX_OUTPUT_BYTES, RequestValue, STATUS_BAD_REQUEST, STATUS_BUDGET_EXCEEDED,
    STATUS_MEMORY_EXCEEDED, STATUS_OK, STATUS_OUTPUT_TOO_SMALL, VALUE_BOOL, VALUE_HTML, VALUE_INT,
    VALUE_TYPED_JSON, VelranResult,
};

const INITIAL_PURE_OUTPUT_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy)]
enum NativeOutputPolicy {
    None,
    RetrySafe,
    SinglePass,
}

pub(super) async fn execute(
    program: &Program,
    native_runtime: Option<&crate::native_runtime::SharedNativeRuntime>,
    route: &Route,
    method: HttpMethod,
    path: &str,
    query_pairs: &[(String, String)],
    form_pairs: &[(String, String)],
    _config: &ServerConfig,
    resource_profiles: &ResourceProfiles,
    _system_values: &[(String, Value)],
    _database: Option<&Database>,
    outbound: Option<&dyn OutboundRuntime>,
    metrics: &Metrics,
) -> Result<AppResponse, AppError> {
    let native_runtime = native_runtime.ok_or(AppError::Internal)?;
    let request = runtime::prepare_native_request_for_route(
        program,
        route,
        method,
        path,
        query_pairs,
        form_pairs,
    )?
    .ok_or(AppError::Internal)?;
    let (limits, _permit) = resource_profiles
        .acquire(route.budget_profile.as_deref())
        .await?;
    let output_policy = native_output_policy(program, route, method);
    execute_prepared_with_host(
        native_runtime,
        request,
        limits.max_instructions,
        limits.max_allocated_bytes,
        limits.max_external_io_bytes,
        outbound,
        metrics,
        output_policy,
    )
}

pub(super) fn execute_prepared(
    native_runtime: &crate::native_runtime::SharedNativeRuntime,
    request: runtime::NativeRequest,
    config: &ServerConfig,
    metrics: &Metrics,
) -> Result<AppResponse, AppError> {
    // Multipart/upload actions currently return scalar JSON or redirects and therefore do not
    // need the caller-owned output buffer. Keep this path allocation-free.
    execute_prepared_with_host(
        native_runtime,
        request,
        config.max_instructions,
        config.max_runtime_alloc_bytes,
        config.max_runtime_alloc_bytes,
        None,
        metrics,
        NativeOutputPolicy::None,
    )
}

fn execute_prepared_with_host(
    native_runtime: &crate::native_runtime::SharedNativeRuntime,
    request: runtime::NativeRequest,
    instruction_budget: u64,
    allocation_budget: u64,
    external_io_budget: u64,
    outbound: Option<&dyn OutboundRuntime>,
    metrics: &Metrics,
    output_policy: NativeOutputPolicy,
) -> Result<AppResponse, AppError> {
    if request.values.len() > MAX_INPUT_FIELDS {
        return Err(AppError::Internal);
    }
    let mut input_storage = [RequestValue::Int(0); MAX_INPUT_FIELDS];
    for (slot, value) in input_storage.iter_mut().zip(&request.values) {
        *slot = as_abi_value(value);
    }
    let inputs = &input_storage[..request.values.len()];
    let mut host_bridge = crate::native_host::NativeHostBridge::new(outbound, external_io_budget);
    let host_api = host_bridge.api();
    let output_limit =
        MAX_OUTPUT_BYTES.min(usize::try_from(allocation_budget).unwrap_or(usize::MAX));
    let initial_len = match output_policy {
        NativeOutputPolicy::None => 0,
        NativeOutputPolicy::RetrySafe => INITIAL_PURE_OUTPUT_BYTES.min(output_limit),
        NativeOutputPolicy::SinglePass => output_limit,
    };
    let mut output = vec![0u8; initial_len];
    let result = native_runtime
        .dispatch_with_host(
            &request.handler_name,
            &host_api,
            inputs,
            &mut output,
            instruction_budget,
            allocation_budget,
        )
        .map_err(|_| AppError::Internal)?;
    if result.status == STATUS_OUTPUT_TOO_SMALL
        && matches!(result.value_tag, VALUE_HTML | VALUE_TYPED_JSON)
    {
        let required = usize::try_from(result.output_len).map_err(|_| AppError::MemoryLimit)?;
        if required > output_limit || result.output_len > allocation_budget {
            return Err(AppError::MemoryLimit);
        }
        if !matches!(output_policy, NativeOutputPolicy::RetrySafe) {
            // Never repeat a handler merely to size its response unless the compiler/runtime
            // metadata proves that the handler is capability-free and DB-free.
            return Err(AppError::MemoryLimit);
        }
        output.resize(required, 0);
        let second = native_runtime
            .dispatch_with_host(
                &request.handler_name,
                &host_api,
                inputs,
                &mut output,
                instruction_budget,
                allocation_budget,
            )
            .map_err(|_| AppError::Internal)?;
        metrics.inc_native_execution();
        return native_result(second, output, output_limit);
    }
    metrics.inc_native_execution();
    native_result(result, output, output_limit)
}

fn native_output_policy(
    program: &Program,
    route: &Route,
    method: HttpMethod,
) -> NativeOutputPolicy {
    if crate::response_kind::route_returns_typed_json(program, route) {
        if method == HttpMethod::Get {
            if let Some(page) = program.page(&route.handler) {
                if !page.needs_db && page.effects.is_empty() {
                    return NativeOutputPolicy::RetrySafe;
                }
            }
        }
        // Typed responses need the caller-owned frame buffer. Effectful handlers must never be
        // re-executed merely to discover response size, so give them one bounded pass.
        return NativeOutputPolicy::SinglePass;
    }
    // Scalar JSON stays in the fixed-width result payload and should not pay for output storage.
    if method != HttpMethod::Get || crate::response_kind::route_returns_json(program, route) {
        return NativeOutputPolicy::None;
    }
    let Some(page) = program.page(&route.handler) else {
        return NativeOutputPolicy::SinglePass;
    };
    if !page.needs_db && page.effects.is_empty() {
        NativeOutputPolicy::RetrySafe
    } else {
        NativeOutputPolicy::SinglePass
    }
}

fn as_abi_value(value: &NativeRequestValue) -> RequestValue<'_> {
    match value {
        NativeRequestValue::Int(v) => RequestValue::Int(*v),
        NativeRequestValue::Bool(v) => RequestValue::Bool(*v),
        NativeRequestValue::String(v) => RequestValue::String(v),
        NativeRequestValue::Email(v) => RequestValue::Email(v),
        NativeRequestValue::Url(v) => RequestValue::Url(v),
        NativeRequestValue::Slug(v) => RequestValue::Slug(v),
        NativeRequestValue::DomainInt { domain, value } => RequestValue::DomainInt {
            domain: *domain,
            value: *value,
        },
        NativeRequestValue::DomainBool { domain, value } => RequestValue::DomainBool {
            domain: *domain,
            value: *value,
        },
        NativeRequestValue::DomainString { domain, value } => RequestValue::DomainString {
            domain: *domain,
            value,
        },
        NativeRequestValue::Upload(v) => RequestValue::Upload(v),
        NativeRequestValue::Image(v) => RequestValue::Image(v),
    }
}

fn native_result(
    result: VelranResult,
    mut output: Vec<u8>,
    output_limit: usize,
) -> Result<AppResponse, AppError> {
    match result.status {
        STATUS_OK => match result.value_tag {
            VALUE_INT => Ok(AppResponse::Json((result.payload as i64).to_string())),
            VALUE_BOOL => Ok(AppResponse::Json(if result.payload == 0 {
                "false".into()
            } else {
                "true".into()
            })),
            VALUE_HTML => {
                let len = usize::try_from(result.output_len).map_err(|_| AppError::Internal)?;
                if len > output.len() {
                    return Err(AppError::Internal);
                }
                output.truncate(len);
                let text = String::from_utf8(output).map_err(|_| AppError::Internal)?;
                Ok(AppResponse::Html(Html::trusted_compiler_output(text)))
            }
            VALUE_TYPED_JSON => {
                let len = usize::try_from(result.output_len).map_err(|_| AppError::Internal)?;
                if len > output.len() || len > output_limit {
                    return Err(AppError::MemoryLimit);
                }
                output.truncate(len);
                let json = crate::typed_json_response::decode_and_serialize(&output, output_limit)?;
                Ok(AppResponse::Json(json))
            }
            _ => Err(AppError::Internal),
        },
        STATUS_BAD_REQUEST => Err(AppError::BadRequest),
        STATUS_BUDGET_EXCEEDED => Err(AppError::InstructionLimit),
        STATUS_MEMORY_EXCEEDED => Err(AppError::MemoryLimit),
        _ => Err(AppError::Internal),
    }
}
