use crate::source_syntax::is_identifier;

pub(crate) fn qualify(namespace: &str, local: &str) -> String {
    if namespace.is_empty() || local.contains("::") {
        local.to_string()
    } else {
        format!("{namespace}::{local}")
    }
}

pub(crate) fn resolve(namespace: &str, name: &str) -> String {
    qualify(namespace, name)
}

pub(crate) fn is_symbol_path(value: &str) -> bool {
    let (path, member) = match value.split_once('.') {
        Some((path, member)) => (path, Some(member)),
        None => (value, None),
    };
    if member.is_some_and(|member| member.contains('.') || !is_identifier(member)) {
        return false;
    }
    let segments: Vec<&str> = path.split("::").collect();
    !segments.is_empty() && segments.iter().all(|segment| is_identifier(segment))
}

pub(crate) fn last_segment(value: &str) -> &str {
    value.rsplit("::").next().unwrap_or(value)
}
