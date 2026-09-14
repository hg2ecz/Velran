use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
pub(super) struct JsonLimits {
    pub(super) max_depth: usize,
    pub(super) max_string_bytes: usize,
    pub(super) max_array_items: usize,
    pub(super) max_object_fields: usize,
}

#[derive(Debug)]
enum BoundedJsonValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<BoundedJsonValue>),
    Object(Vec<(String, BoundedJsonValue)>),
}

#[derive(Clone, Copy)]
struct BoundedSeed {
    limits: JsonLimits,
    depth: usize,
}

impl BoundedSeed {
    fn child<E: de::Error>(self) -> Result<Self, E> {
        let depth = self.depth.saturating_add(1);
        if depth > self.limits.max_depth {
            return Err(E::custom("JSON nesting depth exceeded"));
        }
        Ok(Self {
            limits: self.limits,
            depth,
        })
    }

    fn validate_string<E: de::Error>(&self, value: &str) -> Result<(), E> {
        if value.len() > self.limits.max_string_bytes || value.as_bytes().contains(&0) {
            return Err(E::custom("JSON string too large or invalid"));
        }
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for BoundedSeed {
    type Value = BoundedJsonValue;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for BoundedSeed {
    type Value = BoundedJsonValue;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("bounded JSON")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(BoundedJsonValue::Null)
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(BoundedJsonValue::Null)
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(BoundedJsonValue::Bool(v))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(BoundedJsonValue::Int(v))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        i64::try_from(v)
            .map(BoundedJsonValue::Int)
            .map_err(|_| E::custom("JSON integer out of i64 range"))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        if !v.is_finite() {
            return Err(E::custom("non-finite JSON number"));
        }
        Ok(BoundedJsonValue::Float(v))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        self.validate_string(v)?;
        Ok(BoundedJsonValue::String(v.to_owned()))
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        self.validate_string(&v)?;
        Ok(BoundedJsonValue::String(v))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let child = self.child::<A::Error>()?;
        let mut out = Vec::new();
        while let Some(value) = seq.next_element_seed(child)? {
            if out.len() >= self.limits.max_array_items {
                return Err(de::Error::custom("JSON array item limit exceeded"));
            }
            out.push(value);
        }
        Ok(BoundedJsonValue::Array(out))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let child = self.child::<A::Error>()?;
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if out.len() >= self.limits.max_object_fields {
                return Err(de::Error::custom("JSON object field limit exceeded"));
            }
            self.validate_string::<A::Error>(&key)?;
            if key.is_empty()
                || key.bytes().any(|b| b < 0x20 || b == 0x7f)
                || !seen.insert(key.clone())
            {
                return Err(de::Error::custom("invalid or duplicate JSON field"));
            }
            let value = map.next_value_seed(child)?;
            out.push((key, value));
        }
        Ok(BoundedJsonValue::Object(out))
    }
}

fn flatten_route_json(
    value: BoundedJsonValue,
    max_fields: usize,
    max_field_bytes: usize,
) -> Result<Vec<(String, String)>, ()> {
    let BoundedJsonValue::Object(fields) = value else {
        return Err(());
    };
    if fields.len() > max_fields {
        return Err(());
    }
    let mut out = Vec::new();
    for (key, value) in fields {
        if key.len() > max_field_bytes {
            return Err(());
        }
        match value {
            BoundedJsonValue::String(v) if v.len() <= max_field_bytes => out.push((key, v)),
            BoundedJsonValue::Int(v) => out.push((key, v.to_string())),
            BoundedJsonValue::Bool(v) => {
                out.push((key, if v { "true".into() } else { "false".into() }))
            }
            BoundedJsonValue::Array(values) => {
                if values.is_empty() || values.len() > max_fields {
                    return Err(());
                }
                for value in values {
                    let BoundedJsonValue::String(value) = value else {
                        return Err(());
                    };
                    if value.len() > max_field_bytes {
                        return Err(());
                    }
                    out.push((key.clone(), value));
                }
            }
            BoundedJsonValue::Float(v) => {
                let _ = v;
                return Err(());
            }
            BoundedJsonValue::Null | BoundedJsonValue::Object(_) => return Err(()),
            BoundedJsonValue::String(_) => return Err(()),
        }
    }
    Ok(out)
}

pub(super) fn decode_json_object_bounded(
    body: &[u8],
    max_fields: usize,
    max_field_bytes: usize,
    limits: JsonLimits,
) -> Result<Vec<(String, String)>, ()> {
    if limits.max_depth == 0
        || limits.max_string_bytes == 0
        || limits.max_array_items == 0
        || limits.max_object_fields == 0
    {
        return Err(());
    }
    let mut de = serde_json::Deserializer::from_slice(body);
    let value = BoundedSeed { limits, depth: 0 }
        .deserialize(&mut de)
        .map_err(|_| ())?;
    de.end().map_err(|_| ())?;
    flatten_route_json(value, max_fields, max_field_bytes)
}

pub(super) fn decode_route_json(
    body: &[u8],
    config: &language_core::ServerConfig,
) -> Result<Vec<(String, String)>, ()> {
    decode_json_object_bounded(
        body,
        config.max_form_fields,
        config.max_form_field_bytes,
        JsonLimits {
            max_depth: config.max_json_depth,
            max_string_bytes: config.max_json_string_bytes,
            max_array_items: config.max_json_array_items,
            max_object_fields: config.max_json_object_fields,
        },
    )
}

#[cfg(test)]
pub(super) fn decode_json_object_limited(
    body: &[u8],
    max_fields: usize,
    max_field_bytes: usize,
) -> Result<Vec<(String, String)>, ()> {
    decode_json_object_bounded(
        body,
        max_fields,
        max_field_bytes,
        JsonLimits {
            max_depth: 32,
            max_string_bytes: max_field_bytes.max(1),
            max_array_items: max_fields.max(1),
            max_object_fields: max_fields.max(1),
        },
    )
}
