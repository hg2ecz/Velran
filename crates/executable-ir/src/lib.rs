#![forbid(unsafe_code)]

pub mod shard_planner;

mod model;
mod native_input;
mod native_scalar;
mod pure_model;
mod typed_numeric;
mod verifier;
mod verifier_statements;

pub use model::{
    Capability, EXECUTABLE_IR_VERSION, HandlerManifest, VerifiedExecutableProgram, VerifiedHandler,
    VerifiedHandlerBody,
};
pub use native_scalar::{
    NativeHtmlPart, NativeInputParam, NativeInputType, NativeJsonField, NativeJsonFieldType,
    NativeOutboundCall, NativePureValueType, NativeScalarType, ScalarBinaryOp, ScalarBuiltin,
    ScalarExpr, ScalarStatement, VerifiedScalarBody, statement_fuel,
};
pub use pure_model::VerifiedPureFunction;
pub use typed_numeric::{
    BlockId, CfgNode, CfgNodeKind, IndexProof, LocalId, NumericCollectionKind,
    NumericCollectionType, NumericElementType, NumericType, RangeProof, RangeProofId, TypedExpr,
    TypedExprKind, TypedHtmlPart, TypedInput, TypedLocal, TypedStatement, VerifiedNumericBody,
    typed_statement_fuel,
};
pub use verifier::{VerifyError, verify};
