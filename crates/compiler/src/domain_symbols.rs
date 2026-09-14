use crate::module_namespace::is_symbol_path;
use crate::source_syntax::is_identifier;

pub(crate) fn internal_domain_symbol(name: &str) -> Option<String> {
    if is_symbol_path(name) && !name.contains('.') {
        return Some(name.to_string());
    }
    let (object, member) = name.split_once('.')?;
    if member.contains('.') || !is_symbol_path(object) || !is_identifier(member) {
        return None;
    }
    Some(format!("{object}__{member}"))
}

pub(crate) fn display_domain_symbol(name: &str) -> String {
    match name.rsplit_once("__") {
        Some((object, member)) if is_symbol_path(object) && is_identifier(member) => {
            format!("{object}.{member}")
        }
        _ => name.to_string(),
    }
}
