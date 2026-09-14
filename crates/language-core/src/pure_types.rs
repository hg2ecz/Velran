#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PureValueType {
    Int,
    F32,
    Bool,
    String,
    StringList,
    Struct(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PureReturnType {
    Unit,
    Value(PureValueType),
    Option(PureValueType),
    Result {
        ok: PureValueType,
        err: PureValueType,
    },
}
