use language_core::Value;
use std::collections::BTreeMap;

pub(super) fn estimate_value_bytes(v: &Value) -> u64 {
    match v {
        Value::String(s) => s.len() as u64,
        Value::Email(s) => s.len() as u64,
        Value::Url(s) => s.len() as u64,
        Value::Int(_) => 8,
        Value::F32(_) => 4,
        Value::F32Array(items) => (items.len() as u64).saturating_mul(4),
        Value::StringList(items) => items
            .iter()
            .map(|v| v.len() as u64)
            .sum::<u64>()
            .saturating_add((items.len() as u64).saturating_mul(24)),
        Value::StringDict(items) => estimate_string_dict_bytes(items),
        Value::Bool(_) => 1,
        Value::Date(_) => 10,
        Value::DateTime(_) => 32,
        Value::Uuid(_) => 16,
        Value::Decimal(_) => 16,
        Value::Image(v) => v.canonical().len() as u64,
        Value::Enum { variant, .. } => variant.len() as u64,
        Value::Null => 0,
        Value::List(items) => items
            .iter()
            .map(estimate_value_bytes)
            .sum::<u64>()
            .saturating_add((items.len() * 8) as u64),
        Value::Record(fields) => fields
            .iter()
            .map(|(k, v)| k.len() as u64 + estimate_value_bytes(v))
            .sum::<u64>()
            .saturating_add((fields.len() * 16) as u64),
    }
}

pub(super) fn estimate_string_dict_bytes(items: &BTreeMap<String, String>) -> u64 {
    items
        .iter()
        .map(|(k, v)| (k.len() + v.len()) as u64)
        .sum::<u64>()
        .saturating_add((items.len() as u64).saturating_mul(48))
}
