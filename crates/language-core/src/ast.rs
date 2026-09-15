pub use crate::sum_types::PureSumPredicate;
use crate::values::{F32Value, ValueType};
use crate::web_types::FlashMessage;
use crate::{BuiltinFunction, ObjectAuthorization, PublicError, PublicProjection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    ShiftLeft,
    ShiftRight,
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
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    String(String),
    Int(i64),
    F32(F32Value),
    F32ArrayNew {
        len: Box<Expr>,
        fill: Box<Expr>,
    },
    CollectionIndex {
        collection: String,
        index: Box<Expr>,
    },
    CollectionLen {
        collection: String,
    },
    Bool(bool),
    EnumLiteral {
        enum_id: u16,
        variant: String,
    },
    Slugify(Box<Expr>),
    Builtin {
        function: BuiltinFunction,
        args: Vec<Expr>,
    },
    Variable(String),
    Field {
        base: String,
        field: String,
    },
    PureSumPredicate {
        base: String,
        predicate: PureSumPredicate,
    },
    PureSumUnwrapOr {
        base: String,
        fallback: Box<Expr>,
    },
    Not(Box<Expr>),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlAttrKind {
    Href,
    Action,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlPart {
    Text(String),
    EscapedExpr(Expr),
    SafeHtmlExpr(Expr),
    Image {
        image: Expr,
        alt: Expr,
    },
    Flash,
    RouteAttr {
        kind: HtmlAttrKind,
        route: String,
        args: Vec<Expr>,
    },
    For {
        item: String,
        collection: String,
        template: HtmlTemplate,
    },
    IfSome {
        value: String,
        template: HtmlTemplate,
    },
    ComponentCall {
        component: String,
        args: Vec<Expr>,
    },
    LayoutCall {
        layout: String,
        args: Vec<Expr>,
        content: HtmlTemplate,
    },
    ContentSlot,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlTemplate {
    pub parts: Vec<HtmlPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateParamType {
    Scalar(ValueType),
    Model(String),
    OptionalModel(String),
    ListModel(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateParam {
    pub name: String,
    pub ty: TemplateParamType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentFunction {
    pub name: String,
    pub params: Vec<TemplateParam>,
    pub template: HtmlTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutFunction {
    pub name: String,
    pub params: Vec<TemplateParam>,
    pub template: HtmlTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryCall {
    pub query: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCall {
    pub route: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboundMethod {
    Get,
    PostJson,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundCall {
    pub integration: String,
    pub egress_target: String,
    pub method: OutboundMethod,
    pub path: String,
    pub body: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeStatement {
    Let {
        name: String,
        expr: Expr,
    },
    Set {
        name: String,
        expr: Expr,
    },
    F32ArraySet {
        array: String,
        index: Expr,
        value: Expr,
    },
    StringDictSet {
        dict: String,
        key: Expr,
        value: Expr,
    },
    PureCall {
        target: Option<String>,
        function: String,
        args: Vec<Expr>,
    },
    ReturnStruct {
        schema: String,
        fields: Vec<TypedJsonField>,
    },
    ReturnOption {
        value: Option<Expr>,
    },
    ReturnResult {
        is_ok: bool,
        value: Expr,
    },
    Return(Expr),
    While {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
    If {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageMatchArm {
    pub variant: String,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionMatchArm {
    pub variant: String,
    pub statements: Vec<ActionStatement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedJsonField {
    pub name: String,
    pub ty: crate::ValueType,
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Let {
        name: String,
        expr: Expr,
    },
    LetValidated {
        name: String,
        domain: u16,
        expr: Expr,
    },
    Set {
        name: String,
        expr: Expr,
    },
    While {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
    If {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
    Match {
        expr: Expr,
        enum_id: u16,
        arms: Vec<PageMatchArm>,
    },
    F32ArraySet {
        array: String,
        index: Expr,
        value: Expr,
    },
    StringDictSet {
        dict: String,
        key: Expr,
        value: Expr,
    },
    PureCall {
        target: Option<String>,
        function: String,
        args: Vec<Expr>,
    },
    LetQuery {
        name: String,
        call: QueryCall,
    },
    LetOutboundStatus {
        name: String,
        call: OutboundCall,
    },
    Authorize(ObjectAuthorization),
    Resource {
        profile: String,
        source: SourceLocation,
        statements: Vec<Statement>,
    },
    CanonicalSlug {
        param: String,
        canonical: Expr,
    },
    ReturnHtml(HtmlTemplate),
    ReturnJson(Expr),
    ReturnJsonProjection(PublicProjection),
    ReturnTypedJson {
        schema: String,
        fields: Vec<TypedJsonField>,
    },
    Fail(PublicError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusinessAudit {
    pub object_type: String,
    pub object_id: Expr,
    pub action: String,
    pub previous: Option<Expr>,
    pub new_value: Option<Expr>,
    pub source_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxStatement {
    LetQuery { name: String, call: QueryCall },
    Query(QueryCall),
    BusinessAudit(BusinessAudit),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionStatement {
    Let {
        name: String,
        expr: Expr,
    },
    LetValidated {
        name: String,
        domain: u16,
        expr: Expr,
    },
    Set {
        name: String,
        expr: Expr,
    },
    While {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
    If {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
    Match {
        expr: Expr,
        enum_id: u16,
        arms: Vec<ActionMatchArm>,
    },
    F32ArraySet {
        array: String,
        index: Expr,
        value: Expr,
    },
    StringDictSet {
        dict: String,
        key: Expr,
        value: Expr,
    },
    LetQuery {
        name: String,
        call: QueryCall,
    },
    LetOutboundStatus {
        name: String,
        call: OutboundCall,
    },
    Authorize(ObjectAuthorization),
    Transaction {
        outcome: Option<String>,
        statements: Vec<TxStatement>,
    },
    Resource {
        profile: String,
        source: SourceLocation,
        statements: Vec<ActionStatement>,
    },
    Flash(FlashMessage),
    ReturnRedirect(RouteCall),
    ReturnJson(Expr),
    ReturnJsonProjection(PublicProjection),
    ReturnTypedJson {
        schema: String,
        fields: Vec<TypedJsonField>,
    },
    Fail(PublicError),
}

#[path = "ast_functions.rs"]
mod functions;
pub use functions::*;
