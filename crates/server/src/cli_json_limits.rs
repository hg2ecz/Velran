use crate::server_config_file::FileLimits;
use language_core::ServerConfig;

pub(super) fn apply(limits: &FileLimits, config: &mut ServerConfig) {
    if let Some(v) = limits.max_json_depth {
        config.max_json_depth = v;
    }
    if let Some(v) = limits.max_json_string_bytes {
        config.max_json_string_bytes = v;
    }
    if let Some(v) = limits.max_json_array_items {
        config.max_json_array_items = v;
    }
    if let Some(v) = limits.max_json_object_fields {
        config.max_json_object_fields = v;
    }
}
