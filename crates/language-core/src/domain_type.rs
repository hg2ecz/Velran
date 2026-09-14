use crate::{ValidationKind, ValueType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainType {
    pub name: String,
    pub base: ValueType,
    pub constraints: Vec<ValidationKind>,
}
