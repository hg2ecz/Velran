use language_core::{
    ActionBody, ActionStatement, BinaryOp, BuiltinFunction, ComputeStatement, Expr, FunctionParam,
    HtmlPart, HtmlTemplate, OutboundMethod, PageBody, Program, PureFunction, PureParamType,
    PureReturnType, PureSumPredicate, PureValueType, Statement, ValidationKind, ValueType,
};
use std::collections::BTreeMap;

const MAX_SPLIT_ITEMS: i64 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitXor,
    BitOr,
    LogicalAnd,
    LogicalOr,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    StringConcat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarBuiltin {
    Sin,
    Cos,
    Sqrt,
    MonotonicNanos,
    ToF32,
    StringLen,
    Trim,
    TrimStart,
    TrimEnd,
    Lower,
    Upper,
    Contains,
    StartsWith,
    EndsWith,
    Replace,
    SplitBounded,
    Substring,
    IndexOf,
    LastIndexOf,
    CharAt,
    Repeat,
    DictNew,
    ContainsKey,
    RemoveKey,
    SafeHtmlEmpty,
    SafeHtmlText,
    SafeHtmlElement,
    SafeHtmlLink,
    SafeHtmlConcat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarExpr {
    String(String),
    Int(i64),
    F32(u32),
    Bool(bool),
    Variable(String),
    Not(Box<ScalarExpr>),
    F32ArrayNew {
        len: Box<ScalarExpr>,
        fill: Box<ScalarExpr>,
    },
    Binary {
        left: Box<ScalarExpr>,
        op: ScalarBinaryOp,
        right: Box<ScalarExpr>,
    },
    Builtin {
        function: ScalarBuiltin,
        args: Vec<ScalarExpr>,
    },
    CollectionLen {
        collection: String,
    },
    CollectionIndex {
        collection: String,
        index: Box<ScalarExpr>,
    },
    Field {
        base: String,
        field: String,
        ty: NativeScalarType,
    },
    SumPredicate {
        base: String,
        predicate: PureSumPredicate,
        base_type: NativeScalarType,
    },
    SumUnwrapOr {
        base: String,
        fallback: Box<ScalarExpr>,
        base_type: NativeScalarType,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeHtmlPart {
    Text(String),
    Escaped(ScalarExpr),
    Safe(ScalarExpr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeJsonFieldType {
    Int,
    F32,
    Bool,
    String,
    StringList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeJsonField {
    pub name: String,
    pub ty: NativeJsonFieldType,
    pub expr: ScalarExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarStatement {
    Let {
        name: String,
        expr: ScalarExpr,
    },
    HostOutboundStatus {
        name: String,
        call: NativeOutboundCall,
    },
    Set {
        name: String,
        expr: ScalarExpr,
    },
    F32ArraySet {
        array: String,
        index: ScalarExpr,
        value: ScalarExpr,
    },
    StringDictSet {
        dict: String,
        key: ScalarExpr,
        value: ScalarExpr,
    },
    PureCall {
        target: Option<String>,
        function: String,
        args: Vec<ScalarExpr>,
        param_types: Vec<NativeInputType>,
        array_lens: Vec<u32>,
        return_type: Option<NativeScalarType>,
    },
    If {
        condition: ScalarExpr,
        statements: Vec<ScalarStatement>,
    },
    While {
        condition: ScalarExpr,
        statements: Vec<ScalarStatement>,
    },
    ReturnStruct {
        struct_id: u16,
        fields: Vec<NativeJsonField>,
    },
    ReturnOption {
        inner_type: NativePureValueType,
        value: Option<ScalarExpr>,
    },
    ReturnResult {
        ok_type: NativePureValueType,
        err_type: NativePureValueType,
        is_ok: bool,
        value: ScalarExpr,
    },
    Return(ScalarExpr),
    ReturnHtml(Vec<NativeHtmlPart>),
    ReturnTypedJson {
        schema: String,
        fields: Vec<NativeJsonField>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOutboundCall {
    pub target: String,
    pub path: String,
    pub post_json: bool,
    pub body: Option<ScalarExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeInputType {
    Int,
    F32Array,
    Bool,
    String,
    StringList,
    Struct(u16),
    Email,
    Url,
    Slug,
    DomainInt {
        domain: u16,
        ranges: Vec<(i64, i64)>,
    },
    DomainBool {
        domain: u16,
    },
    DomainString {
        domain: u16,
        lengths: Vec<(usize, usize)>,
    },
    Upload,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeInputParam {
    pub name: String,
    pub ty: NativeInputType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePureValueType {
    Int,
    F32,
    Bool,
    String,
    StringList,
    SafeHtml,
    Struct(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeScalarType {
    Int,
    F32,
    F32Array,
    Bool,
    String,
    StringList,
    StringDict,
    SafeHtml,
    Struct(u16),
    Option(NativePureValueType),
    Result {
        ok: NativePureValueType,
        err: NativePureValueType,
    },
    Upload,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedScalarBody {
    pub(crate) inputs: Vec<NativeInputParam>,
    pub(crate) statements: Vec<ScalarStatement>,
    pub(crate) local_types: BTreeMap<String, NativeScalarType>,
    numeric_body: Option<crate::VerifiedNumericBody>,
}
impl VerifiedScalarBody {
    pub fn inputs(&self) -> &[NativeInputParam] {
        &self.inputs
    }
    pub fn statements(&self) -> &[ScalarStatement] {
        &self.statements
    }
    pub fn local_type(&self, name: &str) -> Option<NativeScalarType> {
        self.local_types.get(name).copied()
    }
    pub fn local_types(&self) -> impl Iterator<Item = NativeScalarType> + '_ {
        self.local_types.values().copied()
    }
    pub fn numeric_body(&self) -> Option<&crate::VerifiedNumericBody> {
        self.numeric_body.as_ref()
    }
    pub fn numeric_fast_path_eligible(&self) -> bool {
        self.numeric_body.is_some()
    }
    pub fn uses_host_api(&self) -> bool {
        statements_use_host_api(&self.statements)
    }
    #[cfg(test)]
    pub(crate) fn test_body(
        inputs: Vec<NativeInputParam>,
        statements: Vec<ScalarStatement>,
        local_types: BTreeMap<String, NativeScalarType>,
    ) -> Self {
        Self {
            inputs,
            statements,
            local_types,
            numeric_body: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn test_numeric_body() -> Self {
        let inputs = vec![NativeInputParam {
            name: "input".into(),
            ty: NativeInputType::Int,
        }];
        let statements = vec![
            ScalarStatement::Let {
                name: "x".into(),
                expr: ScalarExpr::Int(1),
            },
            ScalarStatement::Return(ScalarExpr::Variable("x".into())),
        ];
        let local_types = BTreeMap::from([
            ("input".into(), NativeScalarType::Int),
            ("x".into(), NativeScalarType::Int),
        ]);
        Self {
            inputs,
            statements,
            local_types,
            numeric_body: None,
        }
    }
}

fn statements_use_host_api(statements: &[ScalarStatement]) -> bool {
    statements.iter().any(|statement| match statement {
        ScalarStatement::HostOutboundStatus { .. } => true,
        ScalarStatement::If { statements, .. } | ScalarStatement::While { statements, .. } => {
            statements_use_host_api(statements)
        }
        ScalarStatement::Let { .. }
        | ScalarStatement::Set { .. }
        | ScalarStatement::F32ArraySet { .. }
        | ScalarStatement::StringDictSet { .. }
        | ScalarStatement::PureCall { .. }
        | ScalarStatement::ReturnStruct { .. }
        | ScalarStatement::ReturnOption { .. }
        | ScalarStatement::ReturnResult { .. }
        | ScalarStatement::Return(_)
        | ScalarStatement::ReturnHtml(_)
        | ScalarStatement::ReturnTypedJson { .. } => false,
    })
}

pub fn statement_fuel(statement: &ScalarStatement) -> u64 {
    match statement {
        ScalarStatement::Let { expr, .. }
        | ScalarStatement::Set { expr, .. }
        | ScalarStatement::Return(expr) => 1 + expr_fuel(expr),
        ScalarStatement::ReturnStruct { fields, .. } => {
            1 + fields
                .iter()
                .map(|field| 1 + expr_fuel(&field.expr))
                .sum::<u64>()
        }
        ScalarStatement::ReturnOption { value, .. } => {
            1 + value.as_ref().map(expr_fuel).unwrap_or(0)
        }
        ScalarStatement::ReturnResult { value, .. } => 1 + expr_fuel(value),
        ScalarStatement::HostOutboundStatus { call, .. } => {
            8 + call.body.as_ref().map(expr_fuel).unwrap_or(0)
        }
        ScalarStatement::F32ArraySet { index, value, .. } => {
            1 + expr_fuel(index) + expr_fuel(value)
        }
        ScalarStatement::StringDictSet { key, value, .. } => 2 + expr_fuel(key) + expr_fuel(value),
        ScalarStatement::PureCall { .. } => 1,
        ScalarStatement::ReturnHtml(parts) => {
            1 + parts
                .iter()
                .map(|part| match part {
                    NativeHtmlPart::Text(_) => 1,
                    NativeHtmlPart::Escaped(expr) | NativeHtmlPart::Safe(expr) => {
                        1 + expr_fuel(expr)
                    }
                })
                .sum::<u64>()
        }
        ScalarStatement::ReturnTypedJson { fields, .. } => {
            1 + fields
                .iter()
                .map(|field| 1 + expr_fuel(&field.expr))
                .sum::<u64>()
        }
        ScalarStatement::If { condition, .. } | ScalarStatement::While { condition, .. } => {
            1 + expr_fuel(condition)
        }
    }
}
fn expr_fuel(expr: &ScalarExpr) -> u64 {
    1 + match expr {
        ScalarExpr::String(_)
        | ScalarExpr::Int(_)
        | ScalarExpr::F32(_)
        | ScalarExpr::Bool(_)
        | ScalarExpr::Variable(_)
        | ScalarExpr::CollectionLen { .. }
        | ScalarExpr::Field { .. }
        | ScalarExpr::SumPredicate { .. } => 0,
        ScalarExpr::F32ArrayNew { len, fill } => expr_fuel(len).saturating_add(expr_fuel(fill)),
        ScalarExpr::Not(inner) => expr_fuel(inner),
        ScalarExpr::Binary { left, right, .. } => expr_fuel(left).saturating_add(expr_fuel(right)),
        ScalarExpr::Builtin { function, args } => {
            function_fuel(*function).saturating_add(args.iter().map(expr_fuel).sum::<u64>())
        }
        ScalarExpr::CollectionIndex { index, .. } => expr_fuel(index),
        ScalarExpr::SumUnwrapOr { fallback, .. } => expr_fuel(fallback),
    }
}
fn function_fuel(function: ScalarBuiltin) -> u64 {
    match function {
        ScalarBuiltin::Sin | ScalarBuiltin::Cos | ScalarBuiltin::Sqrt => 15,
        ScalarBuiltin::MonotonicNanos => 3,
        ScalarBuiltin::ToF32 => 1,
        ScalarBuiltin::StringLen => 1,
        ScalarBuiltin::Trim
        | ScalarBuiltin::TrimStart
        | ScalarBuiltin::TrimEnd
        | ScalarBuiltin::Lower
        | ScalarBuiltin::Upper => 2,
        ScalarBuiltin::Contains | ScalarBuiltin::StartsWith | ScalarBuiltin::EndsWith => 2,
        ScalarBuiltin::Replace
        | ScalarBuiltin::SplitBounded
        | ScalarBuiltin::Substring
        | ScalarBuiltin::Repeat => 4,
        ScalarBuiltin::IndexOf | ScalarBuiltin::LastIndexOf => 3,
        ScalarBuiltin::CharAt => 2,
        ScalarBuiltin::DictNew => 1,
        ScalarBuiltin::ContainsKey => 2,
        ScalarBuiltin::RemoveKey => 3,
        ScalarBuiltin::SafeHtmlEmpty => 1,
        ScalarBuiltin::SafeHtmlText => 4,
        ScalarBuiltin::SafeHtmlElement => 4,
        ScalarBuiltin::SafeHtmlLink => 5,
        ScalarBuiltin::SafeHtmlConcat => 3,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScalarLowerError {
    TypeMismatch,
    DuplicateLocal,
}
use NativeScalarType as ScalarType;

pub(crate) fn lower_pure_function(
    function: &PureFunction,
    program: &Program,
) -> Result<VerifiedScalarBody, ScalarLowerError> {
    let inputs = function
        .params
        .iter()
        .map(|param| NativeInputParam {
            name: param.name.clone(),
            ty: match param.ty {
                PureParamType::F32ArrayMut(_) => NativeInputType::F32Array,
                PureParamType::Int => NativeInputType::Int,
                PureParamType::Bool => NativeInputType::Bool,
                PureParamType::Str => NativeInputType::String,
                PureParamType::StringList => NativeInputType::StringList,
                PureParamType::Struct(id) => NativeInputType::Struct(id),
            },
        })
        .collect::<Vec<_>>();
    let mut locals = BTreeMap::new();
    for param in &function.params {
        let ty = match param.ty {
            PureParamType::F32ArrayMut(_) => ScalarType::F32Array,
            PureParamType::Int => ScalarType::Int,
            PureParamType::Bool => ScalarType::Bool,
            PureParamType::Str => ScalarType::String,
            PureParamType::StringList => ScalarType::StringList,
            PureParamType::Struct(id) => ScalarType::Struct(id),
        };
        if locals.insert(param.name.clone(), ty).is_some() {
            return Err(ScalarLowerError::DuplicateLocal);
        }
    }
    let Some(statements) = lower_compute_statements(
        &function.body,
        &locals,
        Some(program),
        Some(&function.return_type),
    )?
    else {
        return Err(ScalarLowerError::TypeMismatch);
    };
    let mut all_types = locals;
    infer_nested_local_types(&statements, &mut all_types)?;
    let mut body = VerifiedScalarBody {
        inputs,
        statements,
        local_types: all_types,
        numeric_body: None,
    };
    let fixed = function
        .params
        .iter()
        .filter_map(|param| match param.ty {
            PureParamType::F32ArrayMut(size) => Some((param.name.clone(), size)),
            PureParamType::Int
            | PureParamType::Bool
            | PureParamType::Str
            | PureParamType::StringList
            | PureParamType::Struct(_) => None,
        })
        .collect();
    let numeric_helper_family = function
        .params
        .iter()
        .any(|param| matches!(param.ty, PureParamType::F32ArrayMut(_)));
    body.numeric_body = if numeric_helper_family {
        let lowered = crate::typed_numeric::lower_with_fixed_inputs(&body, &fixed);
        if lowered.is_none() {
            return Err(ScalarLowerError::TypeMismatch);
        }
        lowered
    } else {
        None
    };
    Ok(body)
}

pub(crate) fn lower_page(
    body: &PageBody,
    params: &[FunctionParam],
    program: &Program,
) -> Result<Option<VerifiedScalarBody>, ScalarLowerError> {
    let PageBody::Statements(statements) = body;
    lower_statements(statements.iter().map(PageOrAction::Page), params, program)
}
pub(crate) fn lower_action(
    body: &ActionBody,
    params: &[FunctionParam],
    program: &Program,
) -> Result<Option<VerifiedScalarBody>, ScalarLowerError> {
    let ActionBody::Statements(statements) = body;
    lower_statements(statements.iter().map(PageOrAction::Action), params, program)
}

fn pure_param_native_type(param: &language_core::PureFunctionParam) -> NativeInputType {
    match param.ty {
        PureParamType::F32ArrayMut(_) => NativeInputType::F32Array,
        PureParamType::Int => NativeInputType::Int,
        PureParamType::Bool => NativeInputType::Bool,
        PureParamType::Str => NativeInputType::String,
        PureParamType::StringList => NativeInputType::StringList,
        PureParamType::Struct(id) => NativeInputType::Struct(id),
    }
}

fn lower_pure_args(
    args: &[Expr],
    pure: &PureFunction,
    locals: &BTreeMap<String, ScalarType>,
    program: &Program,
) -> Result<Vec<ScalarExpr>, ScalarLowerError> {
    if args.len() != pure.params.len() {
        return Err(ScalarLowerError::TypeMismatch);
    }
    let mut lowered = Vec::with_capacity(args.len());
    for (arg, param) in args.iter().zip(&pure.params) {
        // Mutable numeric-kernel parameters are deliberately not general scalar
        // expressions. The frontend has already required explicit `&mut variable`
        // syntax and records that borrow as Expr::Variable. Preserve that contract
        // here instead of routing F32 arrays through lower_expr(), which intentionally
        // rejects arrays as ordinary scalar values.
        if matches!(param.ty, PureParamType::F32ArrayMut(_)) {
            let Expr::Variable(name) = arg else {
                return Err(ScalarLowerError::TypeMismatch);
            };
            if locals.get(name).copied() != Some(ScalarType::F32Array) {
                return Err(ScalarLowerError::TypeMismatch);
            }
            lowered.push(ScalarExpr::Variable(name.clone()));
            continue;
        }

        let Some((expr, actual)) = lower_expr(arg, locals, Some(program))? else {
            return Err(ScalarLowerError::TypeMismatch);
        };
        let expected = match param.ty {
            PureParamType::Int => ScalarType::Int,
            PureParamType::Bool => ScalarType::Bool,
            PureParamType::Str => ScalarType::String,
            PureParamType::StringList => ScalarType::StringList,
            PureParamType::Struct(id) => ScalarType::Struct(id),
            PureParamType::F32ArrayMut(_) => unreachable!("handled above"),
        };
        if actual != expected {
            return Err(ScalarLowerError::TypeMismatch);
        }
        lowered.push(expr);
    }
    Ok(lowered)
}

enum PageOrAction<'a> {
    Page(&'a Statement),
    Action(&'a ActionStatement),
}

fn lower_statements<'a>(
    statements: impl Iterator<Item = PageOrAction<'a>>,
    params: &[FunctionParam],
    program: &Program,
) -> Result<Option<VerifiedScalarBody>, ScalarLowerError> {
    let Some(inputs) = lower_inputs(params, program) else {
        return Ok(None);
    };
    let mut locals = BTreeMap::new();
    for input in &inputs {
        let ty = match &input.ty {
            NativeInputType::Int | NativeInputType::DomainInt { .. } => ScalarType::Int,
            NativeInputType::F32Array => ScalarType::F32Array,
            NativeInputType::Bool | NativeInputType::DomainBool { .. } => ScalarType::Bool,
            NativeInputType::String
            | NativeInputType::Email
            | NativeInputType::Url
            | NativeInputType::Slug
            | NativeInputType::DomainString { .. } => ScalarType::String,
            NativeInputType::StringList => ScalarType::StringList,
            NativeInputType::Struct(id) => ScalarType::Struct(*id),
            NativeInputType::Upload => ScalarType::Upload,
            NativeInputType::Image => ScalarType::Image,
        };
        if locals.insert(input.name.clone(), ty).is_some() {
            return Err(ScalarLowerError::DuplicateLocal);
        }
    }
    let mut out = Vec::new();
    let mut returned = false;
    for statement in statements {
        if returned {
            return Ok(None);
        }
        let lowered = match statement {
            PageOrAction::Page(Statement::Let { name, expr })
            | PageOrAction::Action(ActionStatement::Let { name, expr }) => {
                match lower_let(name, expr, &mut locals, Some(program))? {
                    Some(v) => v,
                    None => return Ok(None),
                }
            }
            PageOrAction::Page(Statement::Set { name, expr })
            | PageOrAction::Action(ActionStatement::Set { name, expr }) => {
                match lower_set(name, expr, &locals, Some(program))? {
                    Some(v) => v,
                    None => return Ok(None),
                }
            }
            PageOrAction::Page(Statement::If {
                condition,
                statements,
            })
            | PageOrAction::Action(ActionStatement::If {
                condition,
                statements,
            }) => match lower_control(false, condition, statements, &locals, Some(program))? {
                Some(v) => v,
                None => return Ok(None),
            },
            PageOrAction::Page(Statement::While {
                condition,
                statements,
            })
            | PageOrAction::Action(ActionStatement::While {
                condition,
                statements,
            }) => match lower_control(true, condition, statements, &locals, Some(program))? {
                Some(v) => v,
                None => return Ok(None),
            },
            PageOrAction::Page(Statement::F32ArraySet {
                array,
                index,
                value,
            })
            | PageOrAction::Action(ActionStatement::F32ArraySet {
                array,
                index,
                value,
            }) => match lower_f32_array_set(array, index, value, &locals, Some(program))? {
                Some(v) => v,
                None => return Ok(None),
            },
            PageOrAction::Page(Statement::StringDictSet { dict, key, value })
            | PageOrAction::Action(ActionStatement::StringDictSet { dict, key, value }) => {
                match lower_string_dict_set(dict, key, value, &locals, Some(program))? {
                    Some(v) => v,
                    None => return Ok(None),
                }
            }
            PageOrAction::Page(Statement::PureCall {
                target,
                function,
                args,
            }) => {
                let Some(pure) = program.pure_function(function) else {
                    return Ok(None);
                };
                let lowered_args = lower_pure_args(args, pure, &locals, program)?;
                let return_type = match (target, &pure.return_type) {
                    (None, PureReturnType::Unit) => None,
                    (Some(name), PureReturnType::Value(PureValueType::Int)) => {
                        if locals.insert(name.clone(), ScalarType::Int).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::Int)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::F32)) => {
                        if locals.insert(name.clone(), ScalarType::F32).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::F32)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::Bool)) => {
                        if locals.insert(name.clone(), ScalarType::Bool).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::Bool)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::String)) => {
                        if locals.insert(name.clone(), ScalarType::String).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::String)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::StringList)) => {
                        if locals
                            .insert(name.clone(), ScalarType::StringList)
                            .is_some()
                        {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::StringList)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::SafeHtml)) => {
                        if locals.insert(name.clone(), ScalarType::SafeHtml).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::SafeHtml)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::Struct(schema))) => {
                        let (id, _) = program
                            .json_schema_by_name(schema)
                            .ok_or(ScalarLowerError::TypeMismatch)?;
                        if locals
                            .insert(name.clone(), ScalarType::Struct(id))
                            .is_some()
                        {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::Struct(id))
                    }
                    (Some(name), PureReturnType::Option(inner)) => {
                        let inner = native_pure_value_type(inner, program)?;
                        let ty = ScalarType::Option(inner);
                        if locals.insert(name.clone(), ty).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ty)
                    }
                    (Some(name), PureReturnType::Result { ok, err }) => {
                        let ty = ScalarType::Result {
                            ok: native_pure_value_type(ok, program)?,
                            err: native_pure_value_type(err, program)?,
                        };
                        if locals.insert(name.clone(), ty).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ty)
                    }
                    _ => return Err(ScalarLowerError::TypeMismatch),
                };
                ScalarStatement::PureCall {
                    target: target.clone(),
                    function: function.clone(),
                    args: lowered_args,
                    param_types: pure.params.iter().map(pure_param_native_type).collect(),
                    array_lens: pure
                        .params
                        .iter()
                        .filter_map(|p| match p.ty {
                            PureParamType::F32ArrayMut(size) => Some(size),
                            PureParamType::Int
                            | PureParamType::Bool
                            | PureParamType::Str
                            | PureParamType::StringList
                            | PureParamType::Struct(_) => None,
                        })
                        .collect(),
                    return_type,
                }
            }
            PageOrAction::Page(Statement::LetOutboundStatus { name, call })
            | PageOrAction::Action(ActionStatement::LetOutboundStatus { name, call }) => {
                let body = match call.body.as_ref() {
                    Some(expr) => match lower_expr(expr, &locals, Some(program))? {
                        Some((expr, _)) => Some(expr),
                        None => return Ok(None),
                    },
                    None => None,
                };
                locals.insert(name.clone(), ScalarType::Int);
                ScalarStatement::HostOutboundStatus {
                    name: name.clone(),
                    call: NativeOutboundCall {
                        target: call.egress_target.clone(),
                        path: call.path.clone(),
                        post_json: matches!(call.method, OutboundMethod::PostJson),
                        body,
                    },
                }
            }
            PageOrAction::Page(Statement::ReturnHtml(template)) => {
                let Some(parts) = lower_html(template, &locals, Some(program))? else {
                    return Ok(None);
                };
                returned = true;
                ScalarStatement::ReturnHtml(parts)
            }
            PageOrAction::Page(Statement::ReturnJson(expr))
            | PageOrAction::Action(ActionStatement::ReturnJson(expr)) => {
                let (expr, ty) = match lower_expr(expr, &locals, Some(program))? {
                    Some(v) => v,
                    None => return Ok(None),
                };
                if !matches!(ty, ScalarType::Int | ScalarType::Bool) {
                    return Ok(None);
                }
                returned = true;
                ScalarStatement::Return(expr)
            }
            PageOrAction::Page(Statement::ReturnTypedJson { schema, fields })
            | PageOrAction::Action(ActionStatement::ReturnTypedJson { schema, fields }) => {
                let mut lowered_fields = Vec::with_capacity(fields.len());
                for field in fields {
                    let (expr, actual_ty) = match lower_expr(&field.expr, &locals, Some(program))? {
                        Some(v) => v,
                        None => return Ok(None),
                    };
                    let expected = match field.ty {
                        ValueType::Int => (ScalarType::Int, NativeJsonFieldType::Int),
                        ValueType::F32 => (ScalarType::F32, NativeJsonFieldType::F32),
                        ValueType::Bool => (ScalarType::Bool, NativeJsonFieldType::Bool),
                        ValueType::String => (ScalarType::String, NativeJsonFieldType::String),
                        ValueType::StringList => {
                            (ScalarType::StringList, NativeJsonFieldType::StringList)
                        }
                        _ => return Ok(None),
                    };
                    if actual_ty != expected.0 {
                        return Err(ScalarLowerError::TypeMismatch);
                    }
                    lowered_fields.push(NativeJsonField {
                        name: field.name.clone(),
                        ty: expected.1,
                        expr,
                    });
                }
                returned = true;
                ScalarStatement::ReturnTypedJson {
                    schema: schema.clone(),
                    fields: lowered_fields,
                }
            }
            _ => return Ok(None),
        };
        out.push(lowered);
    }
    if !returned {
        return Ok(None);
    }
    let mut all_types = locals;
    infer_nested_local_types(&out, &mut all_types)?;
    let mut body = VerifiedScalarBody {
        inputs,
        statements: out,
        local_types: all_types,
        numeric_body: None,
    };
    body.numeric_body = crate::typed_numeric::lower(&body);
    Ok(Some(body))
}

fn infer_nested_local_types(
    statements: &[ScalarStatement],
    locals: &mut BTreeMap<String, NativeScalarType>,
) -> Result<(), ScalarLowerError> {
    for statement in statements {
        match statement {
            ScalarStatement::Let { name, expr } => {
                let ty = lowered_expr_type(expr, locals)?;
                match locals.get(name) {
                    Some(existing) if *existing == ty => {}
                    Some(_) => return Err(ScalarLowerError::TypeMismatch),
                    None => {
                        locals.insert(name.clone(), ty);
                    }
                }
            }
            ScalarStatement::Set { name, expr } => {
                let expected = locals
                    .get(name)
                    .copied()
                    .ok_or(ScalarLowerError::TypeMismatch)?;
                if lowered_expr_type(expr, locals)? != expected {
                    return Err(ScalarLowerError::TypeMismatch);
                }
            }
            ScalarStatement::HostOutboundStatus { name, .. } => match locals.get(name) {
                Some(NativeScalarType::Int) => {}
                Some(_) => return Err(ScalarLowerError::TypeMismatch),
                None => {
                    locals.insert(name.clone(), NativeScalarType::Int);
                }
            },
            ScalarStatement::F32ArraySet {
                array,
                index,
                value,
            } => {
                if locals.get(array).copied() != Some(NativeScalarType::F32Array)
                    || lowered_expr_type(index, locals)? != NativeScalarType::Int
                    || lowered_expr_type(value, locals)? != NativeScalarType::F32
                {
                    return Err(ScalarLowerError::TypeMismatch);
                }
            }
            ScalarStatement::StringDictSet { dict, key, value } => {
                if locals.get(dict).copied() != Some(NativeScalarType::StringDict)
                    || lowered_expr_type(key, locals)? != NativeScalarType::String
                    || lowered_expr_type(value, locals)? != NativeScalarType::String
                {
                    return Err(ScalarLowerError::TypeMismatch);
                }
            }
            ScalarStatement::PureCall {
                target,
                return_type,
                args,
                param_types,
                ..
            } => {
                if let (Some(name), Some(ty)) = (target, return_type) {
                    match locals.get(name) {
                        Some(existing) if *existing == *ty => {}
                        Some(_) => return Err(ScalarLowerError::TypeMismatch),
                        None => {
                            locals.insert(name.clone(), *ty);
                        }
                    }
                }
                if args.len() != param_types.len() {
                    return Err(ScalarLowerError::TypeMismatch);
                }
                for (arg, param_ty) in args.iter().zip(param_types) {
                    let expected = match param_ty {
                        NativeInputType::Int => NativeScalarType::Int,
                        NativeInputType::Bool => NativeScalarType::Bool,
                        NativeInputType::F32Array => NativeScalarType::F32Array,
                        NativeInputType::String => NativeScalarType::String,
                        NativeInputType::StringList => NativeScalarType::StringList,
                        NativeInputType::Struct(id) => NativeScalarType::Struct(*id),
                        _ => return Err(ScalarLowerError::TypeMismatch),
                    };
                    if lowered_expr_type(arg, locals)? != expected {
                        return Err(ScalarLowerError::TypeMismatch);
                    }
                }
            }
            ScalarStatement::If {
                condition,
                statements,
            }
            | ScalarStatement::While {
                condition,
                statements,
            } => {
                if lowered_expr_type(condition, locals)? != NativeScalarType::Bool {
                    return Err(ScalarLowerError::TypeMismatch);
                }
                infer_nested_local_types(statements, locals)?;
            }
            ScalarStatement::Return(expr) => {
                let _ = lowered_expr_type(expr, locals)?;
            }
            ScalarStatement::ReturnOption { value, .. } => {
                if let Some(expr) = value {
                    let _ = lowered_expr_type(expr, locals)?;
                }
            }
            ScalarStatement::ReturnResult { value, .. } => {
                let _ = lowered_expr_type(value, locals)?;
            }
            ScalarStatement::ReturnStruct { fields, .. } => {
                for field in fields {
                    let _ = lowered_expr_type(&field.expr, locals)?;
                }
            }
            ScalarStatement::ReturnHtml(parts) => {
                for part in parts {
                    match part {
                        NativeHtmlPart::Escaped(expr) | NativeHtmlPart::Safe(expr) => {
                            let _ = lowered_expr_type(expr, locals)?;
                        }
                        NativeHtmlPart::Text(_) => {}
                    }
                }
            }
            ScalarStatement::ReturnTypedJson { fields, .. } => {
                for field in fields {
                    let _ = lowered_expr_type(&field.expr, locals)?;
                }
            }
        }
    }
    Ok(())
}

fn lowered_expr_type(
    expr: &ScalarExpr,
    locals: &BTreeMap<String, NativeScalarType>,
) -> Result<NativeScalarType, ScalarLowerError> {
    Ok(match expr {
        ScalarExpr::String(_) => NativeScalarType::String,
        ScalarExpr::Int(_) => NativeScalarType::Int,
        ScalarExpr::F32(_) => NativeScalarType::F32,
        ScalarExpr::Bool(_) => NativeScalarType::Bool,
        ScalarExpr::Variable(name) => locals
            .get(name)
            .copied()
            .ok_or(ScalarLowerError::TypeMismatch)?,
        ScalarExpr::F32ArrayNew { .. } => NativeScalarType::F32Array,
        ScalarExpr::CollectionLen { .. } => NativeScalarType::Int,
        ScalarExpr::Field { ty, .. } => *ty,
        ScalarExpr::SumPredicate { .. } => NativeScalarType::Bool,
        ScalarExpr::SumUnwrapOr { base_type, .. } => match base_type {
            NativeScalarType::Option(inner) => native_pure_value_scalar_type(*inner),
            NativeScalarType::Result { ok, .. } => native_pure_value_scalar_type(*ok),
            _ => return Err(ScalarLowerError::TypeMismatch),
        },
        ScalarExpr::CollectionIndex { collection, index } => match locals
            .get(collection)
            .copied()
            .ok_or(
            ScalarLowerError::TypeMismatch,
        )? {
            NativeScalarType::F32Array => {
                if lowered_expr_type(index, locals)? != NativeScalarType::Int {
                    return Err(ScalarLowerError::TypeMismatch);
                }
                NativeScalarType::F32
            }
            NativeScalarType::StringList => {
                if lowered_expr_type(index, locals)? != NativeScalarType::Int {
                    return Err(ScalarLowerError::TypeMismatch);
                }
                NativeScalarType::String
            }
            NativeScalarType::StringDict => {
                if lowered_expr_type(index, locals)? != NativeScalarType::String {
                    return Err(ScalarLowerError::TypeMismatch);
                }
                NativeScalarType::String
            }
            _ => return Err(ScalarLowerError::TypeMismatch),
        },
        ScalarExpr::Not(_) => NativeScalarType::Bool,
        ScalarExpr::Binary { left, op, .. } => match op {
            ScalarBinaryOp::LogicalAnd
            | ScalarBinaryOp::LogicalOr
            | ScalarBinaryOp::Lt
            | ScalarBinaryOp::Le
            | ScalarBinaryOp::Gt
            | ScalarBinaryOp::Ge
            | ScalarBinaryOp::Eq
            | ScalarBinaryOp::Ne => NativeScalarType::Bool,
            ScalarBinaryOp::StringConcat => NativeScalarType::String,
            _ => lowered_expr_type(left, locals)?,
        },
        ScalarExpr::Builtin { function, .. } => match function {
            ScalarBuiltin::Sin
            | ScalarBuiltin::Cos
            | ScalarBuiltin::Sqrt
            | ScalarBuiltin::ToF32 => NativeScalarType::F32,
            ScalarBuiltin::MonotonicNanos
            | ScalarBuiltin::StringLen
            | ScalarBuiltin::IndexOf
            | ScalarBuiltin::LastIndexOf => NativeScalarType::Int,
            ScalarBuiltin::Contains
            | ScalarBuiltin::StartsWith
            | ScalarBuiltin::EndsWith
            | ScalarBuiltin::ContainsKey => NativeScalarType::Bool,
            ScalarBuiltin::SplitBounded => NativeScalarType::StringList,
            ScalarBuiltin::DictNew | ScalarBuiltin::RemoveKey => NativeScalarType::StringDict,
            ScalarBuiltin::SafeHtmlEmpty
            | ScalarBuiltin::SafeHtmlText
            | ScalarBuiltin::SafeHtmlElement
            | ScalarBuiltin::SafeHtmlLink
            | ScalarBuiltin::SafeHtmlConcat => NativeScalarType::SafeHtml,
            ScalarBuiltin::Trim
            | ScalarBuiltin::TrimStart
            | ScalarBuiltin::TrimEnd
            | ScalarBuiltin::Lower
            | ScalarBuiltin::Upper
            | ScalarBuiltin::Replace
            | ScalarBuiltin::Substring
            | ScalarBuiltin::CharAt
            | ScalarBuiltin::Repeat => NativeScalarType::String,
        },
    })
}

fn lower_html(
    template: &HtmlTemplate,
    locals: &BTreeMap<String, ScalarType>,
    program: Option<&Program>,
) -> Result<Option<Vec<NativeHtmlPart>>, ScalarLowerError> {
    let mut out = Vec::with_capacity(template.parts.len());
    for part in &template.parts {
        match part {
            HtmlPart::Text(text) => out.push(NativeHtmlPart::Text(text.clone())),
            HtmlPart::EscapedExpr(expr) => {
                let Some((expr, ty)) = lower_expr(expr, locals, program)? else {
                    return Ok(None);
                };
                if !matches!(
                    ty,
                    ScalarType::Int | ScalarType::F32 | ScalarType::Bool | ScalarType::String
                ) {
                    return Ok(None);
                }
                out.push(NativeHtmlPart::Escaped(expr));
            }
            HtmlPart::SafeHtmlExpr(expr) => {
                let Some((expr, ty)) = lower_expr(expr, locals, program)? else {
                    return Ok(None);
                };
                if ty != ScalarType::SafeHtml {
                    return Ok(None);
                }
                out.push(NativeHtmlPart::Safe(expr));
            }
            _ => return Ok(None),
        }
    }
    Ok(Some(out))
}

fn lower_inputs(params: &[FunctionParam], program: &Program) -> Option<Vec<NativeInputParam>> {
    if params.len() > runtime_input_limit() {
        return None;
    }
    let mut out = Vec::with_capacity(params.len());
    for param in params {
        let ty = match param.ty {
            ValueType::Int => NativeInputType::Int,
            ValueType::Bool => NativeInputType::Bool,
            ValueType::String => NativeInputType::String,
            ValueType::Email => NativeInputType::Email,
            ValueType::Url => NativeInputType::Url,
            ValueType::Slug => NativeInputType::Slug,
            ValueType::Domain(domain) => lower_domain_input(program, domain)?,
            ValueType::Upload => NativeInputType::Upload,
            ValueType::Image => NativeInputType::Image,
            _ => return None,
        };
        out.push(NativeInputParam {
            name: param.name.clone(),
            ty,
        });
    }
    Some(out)
}

fn lower_domain_input(program: &Program, domain: u16) -> Option<NativeInputType> {
    let definition = program.domain_type_by_id(domain)?;
    match definition.base {
        ValueType::Int => {
            let mut ranges = Vec::new();
            for constraint in &definition.constraints {
                let ValidationKind::Range { min, max } = constraint else {
                    return None;
                };
                ranges.push((*min, *max));
            }
            Some(NativeInputType::DomainInt { domain, ranges })
        }
        ValueType::Bool if definition.constraints.is_empty() => {
            Some(NativeInputType::DomainBool { domain })
        }
        ValueType::String => {
            let mut lengths = Vec::new();
            for constraint in &definition.constraints {
                let ValidationKind::Length { min, max } = constraint else {
                    return None;
                };
                lengths.push((*min, *max));
            }
            Some(NativeInputType::DomainString { domain, lengths })
        }
        _ => None,
    }
}

const fn runtime_input_limit() -> usize {
    64
}

fn lower_compute_statements(
    statements: &[ComputeStatement],
    parent: &BTreeMap<String, ScalarType>,
    program: Option<&Program>,
    pure_return: Option<&PureReturnType>,
) -> Result<Option<Vec<ScalarStatement>>, ScalarLowerError> {
    let mut locals = parent.clone();
    let mut out = Vec::new();
    for statement in statements {
        let lowered = match statement {
            ComputeStatement::Let { name, expr } => lower_let(name, expr, &mut locals, program)?,
            ComputeStatement::Set { name, expr } => lower_set(name, expr, &locals, program)?,
            ComputeStatement::If {
                condition,
                statements,
            } => lower_control(false, condition, statements, &locals, program)?,
            ComputeStatement::While {
                condition,
                statements,
            } => lower_control(true, condition, statements, &locals, program)?,
            ComputeStatement::F32ArraySet {
                array,
                index,
                value,
            } => lower_f32_array_set(array, index, value, &locals, program)?,
            ComputeStatement::StringDictSet { dict, key, value } => {
                lower_string_dict_set(dict, key, value, &locals, program)?
            }
            ComputeStatement::PureCall {
                target,
                function,
                args,
            } => {
                let Some(program) = program else {
                    return Err(ScalarLowerError::TypeMismatch);
                };
                let Some(pure) = program.pure_function(function) else {
                    return Err(ScalarLowerError::TypeMismatch);
                };
                let lowered_args = lower_pure_args(args, pure, &locals, program)?;
                let return_type = match (target, &pure.return_type) {
                    (None, PureReturnType::Unit) => None,
                    (Some(name), PureReturnType::Value(PureValueType::Int)) => {
                        if locals.insert(name.clone(), ScalarType::Int).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::Int)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::F32)) => {
                        if locals.insert(name.clone(), ScalarType::F32).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::F32)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::Bool)) => {
                        if locals.insert(name.clone(), ScalarType::Bool).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::Bool)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::String)) => {
                        if locals.insert(name.clone(), ScalarType::String).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::String)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::StringList)) => {
                        if locals
                            .insert(name.clone(), ScalarType::StringList)
                            .is_some()
                        {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::StringList)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::SafeHtml)) => {
                        if locals.insert(name.clone(), ScalarType::SafeHtml).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::SafeHtml)
                    }
                    (Some(name), PureReturnType::Value(PureValueType::Struct(schema))) => {
                        let (id, _) = program
                            .json_schema_by_name(schema)
                            .ok_or(ScalarLowerError::TypeMismatch)?;
                        if locals
                            .insert(name.clone(), ScalarType::Struct(id))
                            .is_some()
                        {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ScalarType::Struct(id))
                    }
                    (Some(name), PureReturnType::Option(inner)) => {
                        let inner = native_pure_value_type(inner, program)?;
                        let ty = ScalarType::Option(inner);
                        if locals.insert(name.clone(), ty).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ty)
                    }
                    (Some(name), PureReturnType::Result { ok, err }) => {
                        let ty = ScalarType::Result {
                            ok: native_pure_value_type(ok, program)?,
                            err: native_pure_value_type(err, program)?,
                        };
                        if locals.insert(name.clone(), ty).is_some() {
                            return Err(ScalarLowerError::DuplicateLocal);
                        }
                        Some(ty)
                    }
                    _ => return Err(ScalarLowerError::TypeMismatch),
                };
                Some(ScalarStatement::PureCall {
                    target: target.clone(),
                    function: function.clone(),
                    args: lowered_args,
                    param_types: pure.params.iter().map(pure_param_native_type).collect(),
                    array_lens: pure
                        .params
                        .iter()
                        .filter_map(|p| match p.ty {
                            PureParamType::F32ArrayMut(size) => Some(size),
                            PureParamType::Int
                            | PureParamType::Bool
                            | PureParamType::Str
                            | PureParamType::StringList
                            | PureParamType::Struct(_) => None,
                        })
                        .collect(),
                    return_type,
                })
            }
            ComputeStatement::ReturnStruct { schema, fields } => {
                let Some(program) = program else {
                    return Err(ScalarLowerError::TypeMismatch);
                };
                let (struct_id, schema_def) = program
                    .json_schema_by_name(schema)
                    .ok_or(ScalarLowerError::TypeMismatch)?;
                if fields.len() != schema_def.fields.len() {
                    return Err(ScalarLowerError::TypeMismatch);
                }
                let mut lowered = Vec::with_capacity(fields.len());
                for (field, expected_field) in fields.iter().zip(&schema_def.fields) {
                    if field.name != expected_field.name || field.ty != expected_field.ty {
                        return Err(ScalarLowerError::TypeMismatch);
                    }
                    let Some((expr, actual_ty)) = lower_expr(&field.expr, &locals, Some(program))?
                    else {
                        return Ok(None);
                    };
                    let expected = match field.ty {
                        ValueType::Int => (ScalarType::Int, NativeJsonFieldType::Int),
                        ValueType::F32 => (ScalarType::F32, NativeJsonFieldType::F32),
                        ValueType::Bool => (ScalarType::Bool, NativeJsonFieldType::Bool),
                        ValueType::String => (ScalarType::String, NativeJsonFieldType::String),
                        ValueType::StringList => {
                            (ScalarType::StringList, NativeJsonFieldType::StringList)
                        }
                        _ => return Err(ScalarLowerError::TypeMismatch),
                    };
                    if actual_ty != expected.0 {
                        return Err(ScalarLowerError::TypeMismatch);
                    }
                    lowered.push(NativeJsonField {
                        name: field.name.clone(),
                        ty: expected.1,
                        expr,
                    });
                }
                Some(ScalarStatement::ReturnStruct {
                    struct_id,
                    fields: lowered,
                })
            }
            ComputeStatement::ReturnOption { value } => {
                let Some(PureReturnType::Option(inner)) = pure_return else {
                    return Err(ScalarLowerError::TypeMismatch);
                };
                let Some(program) = program else {
                    return Err(ScalarLowerError::TypeMismatch);
                };
                let inner_type = native_pure_value_type(inner, program)?;
                let lowered = match value {
                    Some(expr) => {
                        let Some((expr, actual)) = lower_expr(expr, &locals, Some(program))? else {
                            return Ok(None);
                        };
                        if actual != native_value_scalar_type(inner_type) {
                            return Err(ScalarLowerError::TypeMismatch);
                        }
                        Some(expr)
                    }
                    None => None,
                };
                Some(ScalarStatement::ReturnOption {
                    inner_type,
                    value: lowered,
                })
            }
            ComputeStatement::ReturnResult { is_ok, value } => {
                let Some(PureReturnType::Result { ok, err }) = pure_return else {
                    return Err(ScalarLowerError::TypeMismatch);
                };
                let Some(program) = program else {
                    return Err(ScalarLowerError::TypeMismatch);
                };
                let ok_type = native_pure_value_type(ok, program)?;
                let err_type = native_pure_value_type(err, program)?;
                let expected = if *is_ok { ok_type } else { err_type };
                let Some((value, actual)) = lower_expr(value, &locals, Some(program))? else {
                    return Ok(None);
                };
                if actual != native_value_scalar_type(expected) {
                    return Err(ScalarLowerError::TypeMismatch);
                }
                Some(ScalarStatement::ReturnResult {
                    ok_type,
                    err_type,
                    is_ok: *is_ok,
                    value,
                })
            }
            ComputeStatement::Return(expr) => {
                let Some((expr, _)) = lower_expr(expr, &locals, program)? else {
                    return Ok(None);
                };
                Some(ScalarStatement::Return(expr))
            }
        };
        let Some(lowered) = lowered else {
            return Ok(None);
        };
        out.push(lowered);
    }
    Ok(Some(out))
}

fn native_pure_value_type(
    value: &PureValueType,
    program: &Program,
) -> Result<NativePureValueType, ScalarLowerError> {
    Ok(match value {
        PureValueType::Int => NativePureValueType::Int,
        PureValueType::F32 => NativePureValueType::F32,
        PureValueType::Bool => NativePureValueType::Bool,
        PureValueType::String => NativePureValueType::String,
        PureValueType::StringList => NativePureValueType::StringList,
        PureValueType::SafeHtml => NativePureValueType::SafeHtml,
        PureValueType::Struct(schema) => NativePureValueType::Struct(
            program
                .json_schema_by_name(schema)
                .ok_or(ScalarLowerError::TypeMismatch)?
                .0,
        ),
    })
}

fn native_value_scalar_type(value: NativePureValueType) -> ScalarType {
    match value {
        NativePureValueType::Int => ScalarType::Int,
        NativePureValueType::F32 => ScalarType::F32,
        NativePureValueType::Bool => ScalarType::Bool,
        NativePureValueType::String => ScalarType::String,
        NativePureValueType::StringList => ScalarType::StringList,
        NativePureValueType::SafeHtml => ScalarType::SafeHtml,
        NativePureValueType::Struct(id) => ScalarType::Struct(id),
    }
}

fn lower_let(
    name: &str,
    expr: &Expr,
    locals: &mut BTreeMap<String, ScalarType>,
    program: Option<&Program>,
) -> Result<Option<ScalarStatement>, ScalarLowerError> {
    let Some((expr, ty)) = lower_expr(expr, locals, program)? else {
        return Ok(None);
    };
    if locals.insert(name.to_owned(), ty).is_some() {
        return Err(ScalarLowerError::DuplicateLocal);
    }
    Ok(Some(ScalarStatement::Let {
        name: name.to_owned(),
        expr,
    }))
}
fn lower_set(
    name: &str,
    expr: &Expr,
    locals: &BTreeMap<String, ScalarType>,
    program: Option<&Program>,
) -> Result<Option<ScalarStatement>, ScalarLowerError> {
    let Some(expected) = locals.get(name).copied() else {
        return Err(ScalarLowerError::TypeMismatch);
    };
    let Some((expr, actual)) = lower_expr(expr, locals, program)? else {
        return Ok(None);
    };
    if expected != actual {
        return Err(ScalarLowerError::TypeMismatch);
    }
    Ok(Some(ScalarStatement::Set {
        name: name.to_owned(),
        expr,
    }))
}
fn lower_f32_array_set(
    array: &str,
    index: &Expr,
    value: &Expr,
    locals: &BTreeMap<String, ScalarType>,
    program: Option<&Program>,
) -> Result<Option<ScalarStatement>, ScalarLowerError> {
    if locals.get(array).copied() != Some(ScalarType::F32Array) {
        return Err(ScalarLowerError::TypeMismatch);
    }
    let Some((index, ScalarType::Int)) = lower_expr(index, locals, program)? else {
        return Err(ScalarLowerError::TypeMismatch);
    };
    let Some((value, ScalarType::F32)) = lower_expr(value, locals, program)? else {
        return Err(ScalarLowerError::TypeMismatch);
    };
    Ok(Some(ScalarStatement::F32ArraySet {
        array: array.to_owned(),
        index,
        value,
    }))
}

fn lower_string_dict_set(
    dict: &str,
    key: &Expr,
    value: &Expr,
    locals: &BTreeMap<String, ScalarType>,
    program: Option<&Program>,
) -> Result<Option<ScalarStatement>, ScalarLowerError> {
    if locals.get(dict).copied() != Some(ScalarType::StringDict) {
        return Err(ScalarLowerError::TypeMismatch);
    }
    let Some((key, ScalarType::String)) = lower_expr(key, locals, program)? else {
        return Err(ScalarLowerError::TypeMismatch);
    };
    let Some((value, ScalarType::String)) = lower_expr(value, locals, program)? else {
        return Err(ScalarLowerError::TypeMismatch);
    };
    Ok(Some(ScalarStatement::StringDictSet {
        dict: dict.to_owned(),
        key,
        value,
    }))
}

fn lower_control(
    is_while: bool,
    condition: &Expr,
    statements: &[ComputeStatement],
    locals: &BTreeMap<String, ScalarType>,
    program: Option<&Program>,
) -> Result<Option<ScalarStatement>, ScalarLowerError> {
    let Some((condition, ty)) = lower_expr(condition, locals, program)? else {
        return Ok(None);
    };
    if ty != ScalarType::Bool {
        return Err(ScalarLowerError::TypeMismatch);
    }
    let Some(statements) = lower_compute_statements(statements, locals, program, None)? else {
        return Ok(None);
    };
    Ok(Some(if is_while {
        ScalarStatement::While {
            condition,
            statements,
        }
    } else {
        ScalarStatement::If {
            condition,
            statements,
        }
    }))
}

fn lower_expr(
    expr: &Expr,
    locals: &BTreeMap<String, ScalarType>,
    program: Option<&Program>,
) -> Result<Option<(ScalarExpr, ScalarType)>, ScalarLowerError> {
    Ok(Some(match expr {
        Expr::String(value) => (ScalarExpr::String(value.clone()), ScalarType::String),
        Expr::Int(value) => (ScalarExpr::Int(*value), ScalarType::Int),
        Expr::F32(value) => (ScalarExpr::F32(value.bits()), ScalarType::F32),
        Expr::F32ArrayNew { len, fill } => {
            let Some((len, ScalarType::Int)) = lower_expr(len, locals, program)? else {
                return Err(ScalarLowerError::TypeMismatch);
            };
            let Some((fill, ScalarType::F32)) = lower_expr(fill, locals, program)? else {
                return Err(ScalarLowerError::TypeMismatch);
            };
            (
                ScalarExpr::F32ArrayNew {
                    len: Box::new(len),
                    fill: Box::new(fill),
                },
                ScalarType::F32Array,
            )
        }
        Expr::Bool(value) => (ScalarExpr::Bool(*value), ScalarType::Bool),
        Expr::Field { base, field } => {
            let Some(base_ty) = locals.get(base).copied() else {
                return Err(ScalarLowerError::TypeMismatch);
            };
            let field_ty = match (base_ty, field.as_str()) {
                (ScalarType::Upload, "path" | "filename" | "contentType") => ScalarType::String,
                (ScalarType::Upload, "bytes") => ScalarType::Int,
                (ScalarType::Image, "path" | "contentType") => ScalarType::String,
                (ScalarType::Image, "width" | "height" | "bytes") => ScalarType::Int,
                (ScalarType::Struct(id), field_name) => {
                    let Some(program) = program else {
                        return Ok(None);
                    };
                    let Some(schema) = program.json_schema_by_id(id) else {
                        return Err(ScalarLowerError::TypeMismatch);
                    };
                    let Some(def) = schema
                        .fields
                        .iter()
                        .find(|candidate| candidate.name == field_name)
                    else {
                        return Err(ScalarLowerError::TypeMismatch);
                    };
                    match def.ty {
                        ValueType::Int => ScalarType::Int,
                        ValueType::F32 => ScalarType::F32,
                        ValueType::Bool => ScalarType::Bool,
                        ValueType::String => ScalarType::String,
                        ValueType::StringList => ScalarType::StringList,
                        _ => return Ok(None),
                    }
                }
                _ => return Ok(None),
            };
            (
                ScalarExpr::Field {
                    base: base.clone(),
                    field: field.clone(),
                    ty: field_ty,
                },
                field_ty,
            )
        }
        Expr::Variable(name) => {
            let Some(ty) = locals.get(name).copied() else {
                return Err(ScalarLowerError::TypeMismatch);
            };
            if ty == ScalarType::F32Array {
                return Ok(None);
            }
            (ScalarExpr::Variable(name.clone()), ty)
        }
        Expr::CollectionLen { collection } => {
            let Some(ty) = locals.get(collection).copied() else {
                return Err(ScalarLowerError::TypeMismatch);
            };
            if !matches!(
                ty,
                ScalarType::String
                    | ScalarType::StringList
                    | ScalarType::StringDict
                    | ScalarType::F32Array
            ) {
                return Ok(None);
            }
            (
                ScalarExpr::CollectionLen {
                    collection: collection.clone(),
                },
                ScalarType::Int,
            )
        }
        Expr::CollectionIndex { collection, index } => {
            let Some(collection_ty) = locals.get(collection).copied() else {
                return Ok(None);
            };
            let (result_ty, index_ty) = match collection_ty {
                ScalarType::StringList => (ScalarType::String, ScalarType::Int),
                ScalarType::F32Array => (ScalarType::F32, ScalarType::Int),
                ScalarType::StringDict => (ScalarType::String, ScalarType::String),
                _ => return Ok(None),
            };
            let Some((index, actual_index_ty)) = lower_expr(index, locals, program)? else {
                return Err(ScalarLowerError::TypeMismatch);
            };
            if actual_index_ty != index_ty {
                return Err(ScalarLowerError::TypeMismatch);
            }
            (
                ScalarExpr::CollectionIndex {
                    collection: collection.clone(),
                    index: Box::new(index),
                },
                result_ty,
            )
        }
        Expr::Builtin { function, args } => {
            let Some(function) = lower_builtin(*function) else {
                return Ok(None);
            };
            let mut lowered = Vec::with_capacity(args.len());
            let mut types = Vec::with_capacity(args.len());
            for arg in args {
                let Some((arg, ty)) = lower_expr(arg, locals, program)? else {
                    return Ok(None);
                };
                lowered.push(arg);
                types.push(ty);
            }
            let result = builtin_type(function, &lowered, &types)?;
            (
                ScalarExpr::Builtin {
                    function,
                    args: lowered,
                },
                result,
            )
        }
        Expr::PureSumPredicate { base, predicate } => {
            let Some(base_ty) = locals.get(base).copied() else {
                return Err(ScalarLowerError::TypeMismatch);
            };
            let valid = matches!(
                (base_ty, predicate),
                (
                    ScalarType::Option(_),
                    PureSumPredicate::IsSome | PureSumPredicate::IsNone
                ) | (
                    ScalarType::Result { .. },
                    PureSumPredicate::IsOk | PureSumPredicate::IsErr
                )
            );
            if !valid {
                return Err(ScalarLowerError::TypeMismatch);
            }
            (
                ScalarExpr::SumPredicate {
                    base: base.clone(),
                    predicate: *predicate,
                    base_type: base_ty,
                },
                ScalarType::Bool,
            )
        }
        Expr::PureSumUnwrapOr { base, fallback } => {
            let Some(base_ty) = locals.get(base).copied() else {
                return Err(ScalarLowerError::TypeMismatch);
            };
            let value_type = match base_ty {
                ScalarType::Option(inner) => inner,
                ScalarType::Result { ok, .. } => ok,
                _ => return Err(ScalarLowerError::TypeMismatch),
            };
            if matches!(value_type, NativePureValueType::Struct(_)) {
                return Ok(None);
            }
            let expected = native_pure_value_scalar_type(value_type);
            let Some((fallback, actual)) = lower_expr(fallback, locals, program)? else {
                return Ok(None);
            };
            if actual != expected {
                return Err(ScalarLowerError::TypeMismatch);
            }
            (
                ScalarExpr::SumUnwrapOr {
                    base: base.clone(),
                    fallback: Box::new(fallback),
                    base_type: base_ty,
                },
                expected,
            )
        }
        Expr::Not(inner) => {
            let Some((inner, ty)) = lower_expr(inner, locals, program)? else {
                return Ok(None);
            };
            if ty != ScalarType::Bool {
                return Err(ScalarLowerError::TypeMismatch);
            }
            (ScalarExpr::Not(Box::new(inner)), ScalarType::Bool)
        }
        Expr::Binary { left, op, right } => {
            let Some((left, left_ty)) = lower_expr(left, locals, program)? else {
                return Ok(None);
            };
            let Some((right, right_ty)) = lower_expr(right, locals, program)? else {
                return Ok(None);
            };
            let native_op = if *op == BinaryOp::Add
                && left_ty == ScalarType::String
                && right_ty == ScalarType::String
            {
                ScalarBinaryOp::StringConcat
            } else {
                let Some(native_op) = lower_op(*op) else {
                    return Ok(None);
                };
                native_op
            };
            let result_ty = binary_type(native_op, left_ty, right_ty)?;
            (
                ScalarExpr::Binary {
                    left: Box::new(left),
                    op: native_op,
                    right: Box::new(right),
                },
                result_ty,
            )
        }
        _ => return Ok(None),
    }))
}

fn native_pure_value_scalar_type(value: NativePureValueType) -> ScalarType {
    match value {
        NativePureValueType::Int => ScalarType::Int,
        NativePureValueType::F32 => ScalarType::F32,
        NativePureValueType::Bool => ScalarType::Bool,
        NativePureValueType::String => ScalarType::String,
        NativePureValueType::StringList => ScalarType::StringList,
        NativePureValueType::SafeHtml => ScalarType::SafeHtml,
        NativePureValueType::Struct(id) => ScalarType::Struct(id),
    }
}

fn lower_builtin(function: BuiltinFunction) -> Option<ScalarBuiltin> {
    Some(match function {
        BuiltinFunction::Sin => ScalarBuiltin::Sin,
        BuiltinFunction::Cos => ScalarBuiltin::Cos,
        BuiltinFunction::Sqrt => ScalarBuiltin::Sqrt,
        BuiltinFunction::MonotonicNanos => ScalarBuiltin::MonotonicNanos,
        BuiltinFunction::ToF32 => ScalarBuiltin::ToF32,
        BuiltinFunction::StringLen => ScalarBuiltin::StringLen,
        BuiltinFunction::Trim => ScalarBuiltin::Trim,
        BuiltinFunction::TrimStart => ScalarBuiltin::TrimStart,
        BuiltinFunction::TrimEnd => ScalarBuiltin::TrimEnd,
        BuiltinFunction::Lower => ScalarBuiltin::Lower,
        BuiltinFunction::Upper => ScalarBuiltin::Upper,
        BuiltinFunction::Contains => ScalarBuiltin::Contains,
        BuiltinFunction::StartsWith => ScalarBuiltin::StartsWith,
        BuiltinFunction::EndsWith => ScalarBuiltin::EndsWith,
        BuiltinFunction::Replace => ScalarBuiltin::Replace,
        BuiltinFunction::SplitBounded => ScalarBuiltin::SplitBounded,
        BuiltinFunction::Substring => ScalarBuiltin::Substring,
        BuiltinFunction::IndexOf => ScalarBuiltin::IndexOf,
        BuiltinFunction::LastIndexOf => ScalarBuiltin::LastIndexOf,
        BuiltinFunction::CharAt => ScalarBuiltin::CharAt,
        BuiltinFunction::Repeat => ScalarBuiltin::Repeat,
        BuiltinFunction::DictNew => ScalarBuiltin::DictNew,
        BuiltinFunction::ContainsKey => ScalarBuiltin::ContainsKey,
        BuiltinFunction::RemoveKey => ScalarBuiltin::RemoveKey,
        BuiltinFunction::SafeHtmlEmpty => ScalarBuiltin::SafeHtmlEmpty,
        BuiltinFunction::SafeHtmlText => ScalarBuiltin::SafeHtmlText,
        BuiltinFunction::SafeHtmlElement => ScalarBuiltin::SafeHtmlElement,
        BuiltinFunction::SafeHtmlLink => ScalarBuiltin::SafeHtmlLink,
        BuiltinFunction::SafeHtmlConcat => ScalarBuiltin::SafeHtmlConcat,
        _ => return None,
    })
}

fn builtin_type(
    function: ScalarBuiltin,
    args: &[ScalarExpr],
    types: &[ScalarType],
) -> Result<ScalarType, ScalarLowerError> {
    use ScalarBuiltin::*;
    let exact = |expected: &[ScalarType]| {
        if types == expected {
            Ok(())
        } else {
            Err(ScalarLowerError::TypeMismatch)
        }
    };
    Ok(match function {
        Sin | Cos | Sqrt => {
            exact(&[ScalarType::F32])?;
            ScalarType::F32
        }
        MonotonicNanos => {
            exact(&[])?;
            ScalarType::Int
        }
        ToF32 => {
            exact(&[ScalarType::Int])?;
            ScalarType::F32
        }
        StringLen => {
            exact(&[ScalarType::String])?;
            ScalarType::Int
        }
        Trim | TrimStart | TrimEnd | Lower | Upper => {
            exact(&[ScalarType::String])?;
            ScalarType::String
        }
        Contains | StartsWith | EndsWith => {
            exact(&[ScalarType::String, ScalarType::String])?;
            ScalarType::Bool
        }
        Replace => {
            exact(&[ScalarType::String, ScalarType::String, ScalarType::String])?;
            ScalarType::String
        }
        SplitBounded => {
            exact(&[ScalarType::String, ScalarType::String, ScalarType::Int])?;
            match args.get(2) {
                Some(ScalarExpr::Int(v)) if (1..=MAX_SPLIT_ITEMS).contains(v) => {}
                _ => return Err(ScalarLowerError::TypeMismatch),
            }
            ScalarType::StringList
        }
        Substring => {
            if types != [ScalarType::String, ScalarType::Int]
                && types != [ScalarType::String, ScalarType::Int, ScalarType::Int]
            {
                return Err(ScalarLowerError::TypeMismatch);
            }
            ScalarType::String
        }
        IndexOf | LastIndexOf => {
            exact(&[ScalarType::String, ScalarType::String])?;
            ScalarType::Int
        }
        CharAt => {
            exact(&[ScalarType::String, ScalarType::Int])?;
            ScalarType::String
        }
        Repeat => {
            exact(&[ScalarType::String, ScalarType::Int])?;
            ScalarType::String
        }
        DictNew => {
            exact(&[])?;
            ScalarType::StringDict
        }
        ContainsKey => {
            exact(&[ScalarType::StringDict, ScalarType::String])?;
            ScalarType::Bool
        }
        RemoveKey => {
            exact(&[ScalarType::StringDict, ScalarType::String])?;
            ScalarType::StringDict
        }
        SafeHtmlEmpty => {
            exact(&[])?;
            ScalarType::SafeHtml
        }
        SafeHtmlText => {
            exact(&[ScalarType::String])?;
            ScalarType::SafeHtml
        }
        SafeHtmlElement => {
            exact(&[ScalarType::String, ScalarType::SafeHtml])?;
            ScalarType::SafeHtml
        }
        SafeHtmlLink => {
            exact(&[ScalarType::String, ScalarType::SafeHtml])?;
            ScalarType::SafeHtml
        }
        SafeHtmlConcat => {
            exact(&[ScalarType::SafeHtml, ScalarType::SafeHtml])?;
            ScalarType::SafeHtml
        }
    })
}

fn binary_type(
    op: ScalarBinaryOp,
    left: ScalarType,
    right: ScalarType,
) -> Result<ScalarType, ScalarLowerError> {
    use ScalarBinaryOp::*;
    if op == StringConcat {
        return if left == ScalarType::String && right == ScalarType::String {
            Ok(ScalarType::String)
        } else {
            Err(ScalarLowerError::TypeMismatch)
        };
    }
    if matches!(op, LogicalAnd | LogicalOr) {
        return if left == ScalarType::Bool && right == ScalarType::Bool {
            Ok(ScalarType::Bool)
        } else {
            Err(ScalarLowerError::TypeMismatch)
        };
    }
    if matches!(op, BitAnd | BitXor | BitOr) {
        return if left == ScalarType::Int && right == ScalarType::Int {
            Ok(ScalarType::Int)
        } else {
            Err(ScalarLowerError::TypeMismatch)
        };
    }
    if matches!(op, Eq | Ne) {
        return if left == right
            && matches!(
                left,
                ScalarType::Int | ScalarType::F32 | ScalarType::Bool | ScalarType::String
            ) {
            Ok(ScalarType::Bool)
        } else {
            Err(ScalarLowerError::TypeMismatch)
        };
    }
    if left != right || !matches!(left, ScalarType::Int | ScalarType::F32) {
        return Err(ScalarLowerError::TypeMismatch);
    }
    Ok(match op {
        Lt | Le | Gt | Ge => ScalarType::Bool,
        Add | Sub | Mul | Div | Rem => left,
        _ => unreachable!(),
    })
}

fn lower_op(op: BinaryOp) -> Option<ScalarBinaryOp> {
    Some(match op {
        BinaryOp::Add => ScalarBinaryOp::Add,
        BinaryOp::Sub => ScalarBinaryOp::Sub,
        BinaryOp::Mul => ScalarBinaryOp::Mul,
        BinaryOp::Div => ScalarBinaryOp::Div,
        BinaryOp::Rem => ScalarBinaryOp::Rem,
        BinaryOp::BitAnd => ScalarBinaryOp::BitAnd,
        BinaryOp::BitXor => ScalarBinaryOp::BitXor,
        BinaryOp::BitOr => ScalarBinaryOp::BitOr,
        BinaryOp::LogicalAnd => ScalarBinaryOp::LogicalAnd,
        BinaryOp::LogicalOr => ScalarBinaryOp::LogicalOr,
        BinaryOp::Lt => ScalarBinaryOp::Lt,
        BinaryOp::Le => ScalarBinaryOp::Le,
        BinaryOp::Gt => ScalarBinaryOp::Gt,
        BinaryOp::Ge => ScalarBinaryOp::Ge,
        BinaryOp::Eq => ScalarBinaryOp::Eq,
        BinaryOp::Ne => ScalarBinaryOp::Ne,
        BinaryOp::ShiftLeft | BinaryOp::ShiftRight => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutable_numeric_pure_args_lower_as_explicit_array_variables() {
        let pure = PureFunction {
            name: "fft".into(),
            params: vec![
                language_core::PureFunctionParam {
                    name: "real".into(),
                    ty: PureParamType::F32ArrayMut(4096),
                },
                language_core::PureFunctionParam {
                    name: "imag".into(),
                    ty: PureParamType::F32ArrayMut(4096),
                },
            ],
            return_type: PureReturnType::Unit,
            inline: language_core::InlineHint::Never,
            body: vec![],
        };
        let locals = BTreeMap::from([
            ("real".into(), ScalarType::F32Array),
            ("imag".into(), ScalarType::F32Array),
        ]);
        let args = vec![Expr::Variable("real".into()), Expr::Variable("imag".into())];
        assert_eq!(
            lower_pure_args(&args, &pure, &locals, &Program::default()),
            Ok(vec![
                ScalarExpr::Variable("real".into()),
                ScalarExpr::Variable("imag".into()),
            ])
        );
    }

    #[test]
    fn mutable_numeric_pure_args_reject_array_expressions() {
        let pure = PureFunction {
            name: "kernel".into(),
            params: vec![language_core::PureFunctionParam {
                name: "data".into(),
                ty: PureParamType::F32ArrayMut(16),
            }],
            return_type: PureReturnType::Unit,
            inline: language_core::InlineHint::Unspecified,
            body: vec![],
        };
        let args = vec![Expr::F32ArrayNew {
            len: Box::new(Expr::Int(16)),
            fill: Box::new(Expr::F32(language_core::F32Value::new(0.0).unwrap())),
        }];
        assert_eq!(
            lower_pure_args(&args, &pure, &BTreeMap::new(), &Program::default()),
            Err(ScalarLowerError::TypeMismatch)
        );
    }

    #[test]
    fn mixed_scalar_types_fail_closed() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Int(1)),
            op: BinaryOp::Add,
            right: Box::new(Expr::Bool(true)),
        };
        assert_eq!(
            lower_expr(&expr, &BTreeMap::new(), None),
            Err(ScalarLowerError::TypeMismatch)
        );
    }

    #[test]
    fn bounded_split_requires_compile_time_bound() {
        let expr = Expr::Builtin {
            function: BuiltinFunction::SplitBounded,
            args: vec![
                Expr::String("a,b".into()),
                Expr::String(",".into()),
                Expr::Int(2),
            ],
        };
        assert!(matches!(
            lower_expr(&expr, &BTreeMap::new(), None),
            Ok(Some((ScalarExpr::Builtin { .. }, ScalarType::StringList)))
        ));
    }

    #[test]
    fn string_return_stays_on_vm_until_output_abi_exists() {
        let body = PageBody::Statements(vec![Statement::ReturnJson(Expr::String("hello".into()))]);
        assert_eq!(lower_page(&body, &[], &Program::default()), Ok(None));
    }

    #[test]
    fn bounded_string_request_param_lowers_as_typed_input() {
        let params = vec![FunctionParam::public("q", ValueType::String)];
        let body = PageBody::Statements(vec![Statement::ReturnJson(Expr::Builtin {
            function: BuiltinFunction::StringLen,
            args: vec![Expr::Variable("q".into())],
        })]);
        let lowered = lower_page(&body, &params, &Program::default())
            .expect("valid native input")
            .expect("native eligible");
        assert_eq!(
            lowered.inputs(),
            &[NativeInputParam {
                name: "q".into(),
                ty: NativeInputType::String
            }]
        );
    }

    #[test]
    fn email_and_slug_keep_nominal_input_tags() {
        let params = vec![
            FunctionParam::public("email", ValueType::Email),
            FunctionParam::public("slug", ValueType::Slug),
        ];
        let body = PageBody::Statements(vec![Statement::ReturnJson(Expr::Builtin {
            function: BuiltinFunction::StringLen,
            args: vec![Expr::Variable("slug".into())],
        })]);
        let lowered = lower_page(&body, &params, &Program::default())
            .expect("valid nominal inputs")
            .expect("native eligible");
        assert!(matches!(lowered.inputs()[0].ty, NativeInputType::Email));
        assert!(matches!(lowered.inputs()[1].ty, NativeInputType::Slug));
    }

    #[test]
    fn range_domain_is_revalidated_in_native_ir() {
        let mut program = Program::default();
        program.domain_types.push(language_core::DomainType {
            name: "ArticleId".into(),
            base: ValueType::Int,
            constraints: vec![ValidationKind::Range { min: 1, max: 1000 }],
        });
        let params = vec![FunctionParam::public("id", ValueType::Domain(0))];
        let body = PageBody::Statements(vec![Statement::ReturnJson(Expr::Variable("id".into()))]);
        let lowered = lower_page(&body, &params, &program)
            .expect("valid domain")
            .expect("native eligible");
        assert_eq!(
            lowered.inputs()[0].ty,
            NativeInputType::DomainInt {
                domain: 0,
                ranges: vec![(1, 1000)]
            }
        );
    }
}

#[cfg(test)]
mod native_f32_fft_tests {
    use super::*;

    #[test]
    fn f32_array_and_math_lower_to_native_ir() {
        let mut locals = BTreeMap::new();
        let array = Expr::F32ArrayNew {
            len: Box::new(Expr::Int(4096)),
            fill: Box::new(Expr::F32(language_core::F32Value::new(0.0).unwrap())),
        };
        let Some((_, ScalarType::F32Array)) = lower_expr(&array, &locals, None).unwrap() else {
            panic!("F32 array must lower")
        };
        locals.insert("real".into(), ScalarType::F32Array);
        let value = Expr::Builtin {
            function: BuiltinFunction::Sin,
            args: vec![Expr::F32(language_core::F32Value::new(0.5).unwrap())],
        };
        assert!(matches!(
            lower_f32_array_set("real", &Expr::Int(0), &value, &locals, None),
            Ok(Some(ScalarStatement::F32ArraySet { .. }))
        ));
    }

    #[test]
    fn f32_array_variable_cannot_be_cloned_as_scalar_expression() {
        let mut locals = BTreeMap::new();
        locals.insert("real".into(), ScalarType::F32Array);
        assert_eq!(
            lower_expr(&Expr::Variable("real".into()), &locals, None),
            Ok(None)
        );
    }
}

#[cfg(test)]
mod native_upload_lowering_tests {
    use super::*;
    #[test]
    fn upload_metadata_fields_lower_to_native_scalars() {
        let locals = BTreeMap::from([("file".to_string(), NativeScalarType::Upload)]);
        let path = Expr::Field {
            base: "file".into(),
            field: "path".into(),
        };
        let bytes = Expr::Field {
            base: "file".into(),
            field: "bytes".into(),
        };
        assert!(matches!(
            lower_expr(&path, &locals, None),
            Ok(Some((ScalarExpr::Field { .. }, NativeScalarType::String)))
        ));
        assert!(matches!(
            lower_expr(&bytes, &locals, None),
            Ok(Some((ScalarExpr::Field { .. }, NativeScalarType::Int)))
        ));
    }
}

#[cfg(test)]
mod host_api_detection_tests {
    use super::*;

    #[test]
    fn pure_scalar_statements_do_not_require_host_api() {
        let statements = vec![
            ScalarStatement::Let {
                name: "s".into(),
                expr: ScalarExpr::String("hello".into()),
            },
            ScalarStatement::ReturnHtml(vec![NativeHtmlPart::Escaped(ScalarExpr::Variable(
                "s".into(),
            ))]),
        ];
        assert!(!statements_use_host_api(&statements));
    }

    #[test]
    fn nested_outbound_statement_requires_host_api() {
        let statements = vec![ScalarStatement::If {
            condition: ScalarExpr::Bool(true),
            statements: vec![ScalarStatement::HostOutboundStatus {
                name: "status".into(),
                call: NativeOutboundCall {
                    target: "api".into(),
                    path: "/health".into(),
                    post_json: false,
                    body: None,
                },
            }],
        }];
        assert!(statements_use_host_api(&statements));
    }
}
