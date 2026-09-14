use crate::CredentialPurpose;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    String,
    Email,
    Url,
    Slug,
    Int,
    F32,
    F32Array,
    StringList,
    StringDict,
    Bool,
    Date,
    DateTime,
    Uuid,
    Decimal,
    Image,
    Upload,
    Credential(CredentialPurpose),
    Domain(u16),
    Enum(u16),
}
impl ValueType {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "String" => Some(Self::String),
            "str" | "&str" => Some(Self::String),
            "Email" => Some(Self::Email),
            "Url" => Some(Self::Url),
            "Slug" => Some(Self::Slug),
            "i64" => Some(Self::Int),
            "f32" => Some(Self::F32),
            "Array<f32>" | "Vec<f32>" => Some(Self::F32Array),
            "Vec<String>" => Some(Self::StringList),
            "BTreeMap<String,String>" => Some(Self::StringDict),
            "bool" => Some(Self::Bool),
            "Date" => Some(Self::Date),
            "DateTime" => Some(Self::DateTime),
            "Uuid" => Some(Self::Uuid),
            "Decimal" => Some(Self::Decimal),
            "Image" => Some(Self::Image),
            "Upload" => Some(Self::Upload),
            other => CredentialPurpose::parse(other).map(Self::Credential),
        }
    }
}

pub const TRANSACTION_OUTCOME_ENUM_ID: u16 = u16::MAX;
pub const TRANSACTION_OUTCOME_VARIANTS: [&str; 3] = ["Committed", "RolledBack", "CommitUnknown"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataSensitivity {
    Public,
    Redacted,
    Sensitive,
    Secret,
}

impl Default for DataSensitivity {
    fn default() -> Self {
        Self::Public
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParam {
    pub name: String,
    pub ty: ValueType,
    pub sensitivity: DataSensitivity,
}

impl FunctionParam {
    pub fn public(name: impl Into<String>, ty: ValueType) -> Self {
        Self {
            name: name.into(),
            ty,
            sensitivity: DataSensitivity::Public,
        }
    }
}
pub type PageParam = FunctionParam;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
    pub path: String,
    pub content_type: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}
impl ImageRef {
    pub fn new(
        path: String,
        content_type: String,
        width: u32,
        height: u32,
        bytes: u64,
    ) -> Option<Self> {
        if path.is_empty()
            || path.len() > 512
            || path.starts_with('/')
            || path
                .bytes()
                .any(|b| !(b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.')))
            || path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == ".." || p.starts_with('.'))
        {
            return None;
        }
        if !matches!(content_type.as_str(), "image/png" | "image/jpeg")
            || width == 0
            || height == 0
            || bytes == 0
        {
            return None;
        }
        Some(Self {
            path,
            content_type,
            width,
            height,
            bytes,
        })
    }
    pub fn canonical(&self) -> String {
        format!(
            "img1;{};{};{};{};{}",
            self.path, self.content_type, self.width, self.height, self.bytes
        )
    }
    pub fn parse(raw: &str) -> Option<Self> {
        let mut it = raw.split(';');
        if it.next()? != "img1" {
            return None;
        }
        let path = it.next()?.to_string();
        let content_type = it.next()?.to_string();
        let width = it.next()?.parse().ok()?;
        let height = it.next()?.parse().ok()?;
        let bytes = it.next()?.parse().ok()?;
        if it.next().is_some() {
            return None;
        }
        Self::new(path, content_type, width, height, bytes)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct F32Value(u32);
impl F32Value {
    pub fn new(value: f32) -> Option<Self> {
        value.is_finite().then(|| Self(value.to_bits()))
    }
    pub fn get(self) -> f32 {
        f32::from_bits(self.0)
    }
    pub fn bits(self) -> u32 {
        self.0
    }
    pub fn from_bits(bits: u32) -> Option<Self> {
        Self::new(f32::from_bits(bits))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Email(String),
    Url(String),
    Int(i64),
    F32(F32Value),
    F32Array(Vec<F32Value>),
    StringList(Vec<String>),
    StringDict(BTreeMap<String, String>),
    Bool(bool),
    Date(NaiveDate),
    DateTime(DateTime<Utc>),
    Uuid(Uuid),
    Decimal(Decimal),
    Image(ImageRef),
    Enum { enum_id: u16, variant: String },
    Null,
    Record(HashMap<String, Value>),
    List(Vec<Value>),
}
impl Value {
    pub fn ty(&self) -> Option<ValueType> {
        match self {
            Self::String(_) => Some(ValueType::String),
            Self::Email(_) => Some(ValueType::Email),
            Self::Url(_) => Some(ValueType::Url),
            Self::Int(_) => Some(ValueType::Int),
            Self::F32(_) => Some(ValueType::F32),
            Self::F32Array(_) => Some(ValueType::F32Array),
            Self::StringList(_) => Some(ValueType::StringList),
            Self::StringDict(_) => Some(ValueType::StringDict),
            Self::Bool(_) => Some(ValueType::Bool),
            Self::Date(_) => Some(ValueType::Date),
            Self::DateTime(_) => Some(ValueType::DateTime),
            Self::Uuid(_) => Some(ValueType::Uuid),
            Self::Decimal(_) => Some(ValueType::Decimal),
            Self::Image(_) => Some(ValueType::Image),
            Self::Enum { enum_id, .. } => Some(ValueType::Enum(*enum_id)),
            Self::Null | Self::Record(_) | Self::List(_) => None,
        }
    }
    pub fn display_text(&self) -> Option<String> {
        match self {
            Self::String(v) => Some(v.clone()),
            Self::Email(v) => Some(v.clone()),
            Self::Url(v) => Some(v.clone()),
            Self::Int(v) => Some(v.to_string()),
            Self::F32(v) => Some(v.get().to_string()),
            Self::F32Array(_) => None,
            Self::StringList(_) => None,
            Self::StringDict(_) => None,
            Self::Bool(v) => Some(v.to_string()),
            Self::Date(v) => Some(v.format("%Y-%m-%d").to_string()),
            Self::DateTime(v) => Some(v.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)),
            Self::Uuid(v) => Some(v.hyphenated().to_string()),
            Self::Decimal(v) => Some(v.normalize().to_string()),
            Self::Image(v) => Some(v.canonical()),
            Self::Enum { variant, .. } => Some(variant.clone()),
            Self::Null => Some(String::new()),
            Self::Record(_) | Self::List(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{F32Value, ImageRef, ValueType};

    #[test]
    fn value_type_parser_rejects_legacy_spellings() {
        assert_eq!(ValueType::parse("Int"), None);
        assert_eq!(ValueType::parse("Bool"), None);
        assert_eq!(ValueType::parse("F32"), None);
        assert_eq!(ValueType::parse("Array<F32>"), None);
        assert_eq!(ValueType::parse("List<String>"), None);
        assert_eq!(ValueType::parse("Dict<String,String>"), None);
    }

    #[test]
    fn f32_value_has_stable_u32_abi_layout() {
        assert_eq!(std::mem::size_of::<F32Value>(), std::mem::size_of::<u32>());
        assert_eq!(
            std::mem::align_of::<F32Value>(),
            std::mem::align_of::<u32>()
        );
    }

    #[test]
    fn image_ref_round_trips_and_rejects_unsafe_path() {
        let x = ImageRef::new(
            "media/0123456789abcdef0123456789abcdef.upload".into(),
            "image/png".into(),
            640,
            480,
            12345,
        )
        .unwrap();
        assert_eq!(ImageRef::parse(&x.canonical()), Some(x));
        assert!(ImageRef::new("../secret".into(), "image/png".into(), 1, 1, 1).is_none());
        assert!(
            ImageRef::new("media/.private/x.png".into(), "image/png".into(), 1, 1, 1).is_none()
        );
        assert!(ImageRef::new(".hidden.png".into(), "image/png".into(), 1, 1, 1).is_none());
        assert!(ImageRef::new("media/x".into(), "image/svg+xml".into(), 1, 1, 1).is_none());
    }
}
