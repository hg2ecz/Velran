use super::{ActionStatement, ComputeStatement, Statement};
use crate::PureReturnType;
use crate::values::FunctionParam;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub function: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUse {
    pub profile: String,
    pub source: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageBody {
    Statements(Vec<Statement>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionBody {
    Statements(Vec<ActionStatement>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineHint {
    Unspecified,
    Hint,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PureParamType {
    F32ArrayMut(u32),
    Str,
    StringList,
    Struct(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InherentMethod {
    pub target: String,
    pub name: String,
    pub function: String,
    pub has_receiver: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PureFunctionParam {
    pub name: String,
    pub ty: PureParamType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PureFunction {
    pub name: String,
    pub params: Vec<PureFunctionParam>,
    pub return_type: PureReturnType,
    pub inline: InlineHint,
    pub body: Vec<ComputeStatement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageFunction {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub needs_db: bool,
    pub effects: Vec<crate::Effect>,
    pub security: crate::HandlerSecurityContract,
    pub body: PageBody,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionFunction {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub needs_db: bool,
    pub effects: Vec<crate::Effect>,
    pub security: crate::HandlerSecurityContract,
    pub body: ActionBody,
}
