use language_core::{AppError, FormField, Value, ValueType};
use std::collections::{HashMap, HashSet};

const ABSOLUTE_COLLECTION_MAX: usize = 256;

pub(crate) fn group_fields(
    schema: &[FormField],
    pairs: &[(String, String)],
) -> Result<HashMap<String, Vec<String>>, AppError> {
    let expected: HashMap<&str, ValueType> = schema
        .iter()
        .map(|field| (field.name.as_str(), field.ty))
        .collect();
    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    let mut scalar_seen = HashSet::new();
    for (name, value) in pairs {
        let ty = expected.get(name.as_str()).ok_or(AppError::BadRequest)?;
        if *ty != ValueType::StringList && !scalar_seen.insert(name.as_str()) {
            return Err(AppError::BadRequest);
        }
        let values = grouped.entry(name.clone()).or_default();
        if values.len() >= ABSOLUTE_COLLECTION_MAX {
            return Err(AppError::BadRequest);
        }
        values.push(value.clone());
    }
    Ok(grouped)
}

pub(crate) fn decode_string_list(values: &[String]) -> Value {
    Value::StringList(values.to_vec())
}
