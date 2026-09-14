use crate::module_namespace::resolve;
use language_core::{DataSensitivity, Program, ValueType};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnnotatedValueType {
    pub value_type: ValueType,
    pub sensitivity: DataSensitivity,
}
pub(crate) fn resolve_value_type(
    raw: &str,
    namespace: &str,
    program: &Program,
) -> Option<ValueType> {
    resolve_annotated_value_type(raw, namespace, program).map(|resolved| resolved.value_type)
}
pub(crate) fn resolve_annotated_value_type(
    raw: &str,
    namespace: &str,
    program: &Program,
) -> Option<AnnotatedValueType> {
    let (sensitivity, inner) = split_sensitivity_wrapper(raw)?;
    let value_type = ValueType::parse(inner).or_else(|| {
        let symbol = resolve(namespace, inner);
        program
            .domain_type_by_name(&symbol)
            .map(|(id, _)| ValueType::Domain(id))
            .or_else(|| {
                program
                    .enum_by_name(&symbol)
                    .filter(|_| program.symbol_is_accessible_from(&symbol, namespace))
                    .map(|(id, _)| ValueType::Enum(id))
            })
    })?;
    Some(AnnotatedValueType {
        value_type,
        sensitivity,
    })
}
fn split_sensitivity_wrapper(raw: &str) -> Option<(DataSensitivity, &str)> {
    if let Some(inner) = crate::generic_type_syntax::unwrap_generic(raw, "Secret") {
        return Some((DataSensitivity::Secret, inner));
    }
    if let Some(inner) = crate::generic_type_syntax::unwrap_generic(raw, "Sensitive") {
        return Some((DataSensitivity::Sensitive, inner));
    }
    if raw.starts_with("Secret<") || raw.starts_with("Sensitive<") {
        return None;
    }
    Some((DataSensitivity::Public, raw))
}
