use language_core::AppError;
use observability::server_event;

pub(super) fn log_runtime_error(
    error: &AppError,
    request_id: &str,
    domain_namespace: &str,
    route_name: &str,
    method: &str,
    path: &str,
) {
    let (level, event) = match error {
        AppError::InstructionLimit => ("warn", "app_instruction_limit"),
        AppError::MemoryLimit => ("warn", "app_memory_limit"),
        AppError::DeadlineExceeded => ("warn", "app_deadline_exceeded"),
        AppError::ExternalIoLimit => ("warn", "app_external_io_limit"),
        AppError::Database => ("error", "app_database_error"),
        AppError::Internal => ("error", "app_internal_error"),
        _ => return,
    };
    server_event(
        level,
        event,
        "runtime",
        &format!(
            "request_id={request_id} domain={domain_namespace} route={route_name} method={method} path={path} error={error}"
        ),
    );
}
