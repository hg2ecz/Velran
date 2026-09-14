use crate::{FunctionParam, ValueType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub name: String,
    pub fields: Vec<FunctionParam>,
    pub tenant_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormField {
    pub name: String,
    pub ty: ValueType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormSchema {
    pub name: String,
    pub fields: Vec<FormField>,
    pub validations: Vec<ValidationRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonSchema {
    pub name: String,
    pub fields: Vec<FormField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormFieldIssue {
    pub field: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormFailure {
    pub schema: String,
    pub values: Vec<(String, String)>,
    pub issues: Vec<FormFieldIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationKind {
    Length { min: usize, max: usize },
    Range { min: i64, max: i64 },
    Items { min: usize, max: usize },
    Pattern { regex: String },
    SameAs { other: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationRule {
    pub field: String,
    pub kind: ValidationKind,
}
