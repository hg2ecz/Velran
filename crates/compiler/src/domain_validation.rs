use crate::module_namespace::resolve;
use language_core::{Program, ValidationRule};

pub(super) fn rules_for_binding(
    raw_binding: &str,
    field: &str,
    namespace: &str,
    program: &Program,
) -> Vec<ValidationRule> {
    let Some(open) = raw_binding.find('<') else {
        return Vec::new();
    };
    let Some(type_name) = raw_binding.get(open + 1..raw_binding.len().saturating_sub(1)) else {
        return Vec::new();
    };
    rules_for_type(type_name, field, namespace, program)
}

pub(super) fn rules_for_type(
    type_name: &str,
    field: &str,
    namespace: &str,
    program: &Program,
) -> Vec<ValidationRule> {
    let symbol = resolve(namespace, type_name);
    program
        .domain_type(&symbol)
        .map(|domain| {
            domain
                .constraints
                .iter()
                .cloned()
                .map(|kind| ValidationRule {
                    field: field.to_string(),
                    kind,
                })
                .collect()
        })
        .unwrap_or_default()
}
