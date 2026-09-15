use crate::{
    NativeInputParam, NativeScalarType, ScalarBinaryOp, ScalarBuiltin, ScalarExpr, ScalarStatement,
    VerifiedScalarBody,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericElementType {
    F32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericCollectionKind {
    Array,
    FixedArray(u32),
    SharedSlice,
    MutableSlice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericCollectionType {
    pub element: NumericElementType,
    pub kind: NumericCollectionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericType {
    I64,
    F32,
    Bool,
    Collection(NumericCollectionType),
}

impl NumericType {
    pub const ARRAY_F32: Self = Self::Collection(NumericCollectionType {
        element: NumericElementType::F32,
        kind: NumericCollectionKind::Array,
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RangeProofId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeProof {
    pub id: RangeProofId,
    pub index: LocalId,
    pub collection: LocalId,
    pub lower_inclusive: i64,
    pub upper_exclusive: i64,
    pub step: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexProof {
    RuntimeChecked,
    Proven(RangeProofId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedLocal {
    pub id: LocalId,
    pub debug_name: String,
    pub ty: NumericType,
    pub is_input: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedInput {
    pub param: NativeInputParam,
    pub local: LocalId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedExpr {
    pub ty: NumericType,
    pub kind: TypedExprKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedExprKind {
    Int(i64),
    F32(u32),
    Bool(bool),
    Local(LocalId),
    Not(Box<TypedExpr>),
    ArrayNew {
        element: NumericElementType,
        len: Box<TypedExpr>,
        fill: Box<TypedExpr>,
    },
    Binary {
        left: Box<TypedExpr>,
        op: ScalarBinaryOp,
        right: Box<TypedExpr>,
    },
    Builtin {
        function: ScalarBuiltin,
        args: Vec<TypedExpr>,
    },
    CollectionLen {
        collection: LocalId,
    },
    CollectionIndex {
        collection: LocalId,
        index: Box<TypedExpr>,
        proof: IndexProof,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedHtmlPart {
    Text(String),
    Escaped(TypedExpr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedStatement {
    Let {
        local: LocalId,
        expr: TypedExpr,
    },
    Set {
        local: LocalId,
        expr: TypedExpr,
    },
    ArraySet {
        array: LocalId,
        index: TypedExpr,
        value: TypedExpr,
        proof: IndexProof,
    },
    PureCall {
        target: Option<LocalId>,
        function: String,
        args: Vec<LocalId>,
        array_lens: Vec<u32>,
        return_type: Option<NumericType>,
    },
    If {
        condition: TypedExpr,
        statements: Vec<TypedStatement>,
    },
    While {
        condition: TypedExpr,
        statements: Vec<TypedStatement>,
    },
    Return(TypedExpr),
    ReturnHtml(Vec<TypedHtmlPart>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfgNodeKind {
    Operation,
    Branch,
    LoopCondition,
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfgNode {
    pub id: BlockId,
    pub kind: CfgNodeKind,
    pub successors: Vec<BlockId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedNumericBody {
    inputs: Vec<TypedInput>,
    locals: Vec<TypedLocal>,
    statements: Vec<TypedStatement>,
    cfg: Vec<CfgNode>,
    entry: Option<BlockId>,
    range_proofs: Vec<RangeProof>,
}

impl VerifiedNumericBody {
    pub fn inputs(&self) -> &[TypedInput] {
        &self.inputs
    }
    pub fn locals(&self) -> &[TypedLocal] {
        &self.locals
    }
    pub fn statements(&self) -> &[TypedStatement] {
        &self.statements
    }
    pub fn cfg(&self) -> &[CfgNode] {
        &self.cfg
    }
    pub fn entry(&self) -> Option<BlockId> {
        self.entry
    }
    pub fn range_proofs(&self) -> &[RangeProof] {
        &self.range_proofs
    }
    pub fn range_proof(&self, id: RangeProofId) -> &RangeProof {
        &self.range_proofs[id.0 as usize]
    }
    pub fn local(&self, id: LocalId) -> &TypedLocal {
        &self.locals[id.0 as usize]
    }
}

fn numeric_type(ty: NativeScalarType) -> Option<NumericType> {
    Some(match ty {
        NativeScalarType::Int => NumericType::I64,
        NativeScalarType::F32 => NumericType::F32,
        NativeScalarType::F32Array => NumericType::ARRAY_F32,
        NativeScalarType::Bool => NumericType::Bool,
        NativeScalarType::String
        | NativeScalarType::SafeHtml
        | NativeScalarType::StringList
        | NativeScalarType::StringDict
        | NativeScalarType::Struct(_)
        | NativeScalarType::Option(_)
        | NativeScalarType::Result { .. }
        | NativeScalarType::Upload
        | NativeScalarType::Image => return None,
    })
}

pub(crate) fn lower(body: &VerifiedScalarBody) -> Option<VerifiedNumericBody> {
    lower_impl(body, &BTreeMap::new())
}

pub(crate) fn lower_with_fixed_inputs(
    body: &VerifiedScalarBody,
    fixed_inputs: &BTreeMap<String, u32>,
) -> Option<VerifiedNumericBody> {
    lower_impl(body, fixed_inputs)
}

fn lower_impl(
    body: &VerifiedScalarBody,
    fixed_inputs: &BTreeMap<String, u32>,
) -> Option<VerifiedNumericBody> {
    if !body.local_types.values().all(|ty| {
        matches!(
            ty,
            NativeScalarType::Int
                | NativeScalarType::F32
                | NativeScalarType::F32Array
                | NativeScalarType::Bool
        )
    }) {
        return None;
    }

    let mut ids = BTreeMap::<String, LocalId>::new();
    let mut locals = Vec::<TypedLocal>::new();
    let mut inputs = Vec::<TypedInput>::new();

    for input in &body.inputs {
        let ty = numeric_type(*body.local_types.get(&input.name)?)?;
        let id = intern_local(&input.name, ty, true, &mut ids, &mut locals)?;
        if let Some(size) = fixed_inputs.get(&input.name).copied() {
            locals[id.0 as usize].ty = NumericType::Collection(NumericCollectionType {
                element: NumericElementType::F32,
                kind: NumericCollectionKind::FixedArray(size),
            });
        }
        inputs.push(TypedInput {
            param: input.clone(),
            local: id,
        });
    }
    collect_declared_locals(&body.statements, &body.local_types, &mut ids, &mut locals)?;

    let mut statements = body
        .statements
        .iter()
        .map(|s| lower_statement(s, body, &ids))
        .collect::<Option<Vec<_>>>()?;
    promote_constant_arrays(&mut statements, &mut locals);
    if !validate_pure_calls(&statements, &locals) {
        return None;
    }
    let range_proofs = analyze_ranges(&mut statements, &locals);
    let mut builder = CfgBuilder::default();
    let (entry, _) = builder.sequence(&statements, None);

    Some(VerifiedNumericBody {
        inputs,
        locals,
        statements,
        cfg: builder.nodes,
        entry,
        range_proofs,
    })
}

fn promote_constant_arrays(statements: &[TypedStatement], locals: &mut [TypedLocal]) {
    fn collect(statements: &[TypedStatement], out: &mut Vec<(LocalId, u32)>) {
        for statement in statements {
            match statement {
                TypedStatement::Let { local, expr } => {
                    if let TypedExprKind::ArrayNew { len, .. } = &expr.kind {
                        if let TypedExprKind::Int(value) = &len.kind {
                            if let Ok(size) = u32::try_from(*value) {
                                // Keep large/zero arrays on the Vec path. 16K f32 values are
                                // 64 KiB, which is a deliberately conservative per-local stack cap.
                                if (1..=16_384).contains(&size) {
                                    out.push((*local, size));
                                }
                            }
                        }
                    }
                }
                TypedStatement::If { statements, .. }
                | TypedStatement::While { statements, .. } => collect(statements, out),
                _ => {}
            }
        }
    }

    let mut candidates = Vec::new();
    collect(statements, &mut candidates);
    for (local, size) in candidates {
        // Whole-collection reassignment keeps Vec semantics for now; element writes are fine.
        if writes_local(statements, local) {
            continue;
        }
        locals[local.0 as usize].ty = NumericType::Collection(NumericCollectionType {
            element: NumericElementType::F32,
            kind: NumericCollectionKind::FixedArray(size),
        });
    }
}

fn validate_pure_calls(statements: &[TypedStatement], locals: &[TypedLocal]) -> bool {
    statements.iter().all(|statement| match statement {
        TypedStatement::PureCall { args, array_lens, .. } => {
            args.len() == array_lens.len() && args.iter().zip(array_lens).all(|(arg, expected)| {
                matches!(locals[arg.0 as usize].ty, NumericType::Collection(NumericCollectionType { element: NumericElementType::F32, kind: NumericCollectionKind::FixedArray(actual) }) if actual == *expected)
            })
        }
        TypedStatement::If { statements, .. } | TypedStatement::While { statements, .. } => validate_pure_calls(statements, locals),
        _ => true,
    })
}

fn intern_local(
    name: &str,
    ty: NumericType,
    is_input: bool,
    ids: &mut BTreeMap<String, LocalId>,
    locals: &mut Vec<TypedLocal>,
) -> Option<LocalId> {
    if ids.contains_key(name) {
        return None;
    }
    let id = LocalId(u32::try_from(locals.len()).ok()?);
    ids.insert(name.to_owned(), id);
    locals.push(TypedLocal {
        id,
        debug_name: name.to_owned(),
        ty,
        is_input,
    });
    Some(id)
}

fn collect_declared_locals(
    statements: &[ScalarStatement],
    types: &BTreeMap<String, NativeScalarType>,
    ids: &mut BTreeMap<String, LocalId>,
    locals: &mut Vec<TypedLocal>,
) -> Option<()> {
    for statement in statements {
        match statement {
            ScalarStatement::Let { name, .. } => {
                if !ids.contains_key(name) {
                    intern_local(name, numeric_type(*types.get(name)?)?, false, ids, locals)?;
                }
            }
            ScalarStatement::PureCall {
                target: Some(name), ..
            } => {
                if !ids.contains_key(name) {
                    intern_local(name, numeric_type(*types.get(name)?)?, false, ids, locals)?;
                }
            }
            ScalarStatement::If { statements, .. } | ScalarStatement::While { statements, .. } => {
                collect_declared_locals(statements, types, ids, locals)?
            }
            _ => {}
        }
    }
    Some(())
}

fn lower_statement(
    statement: &ScalarStatement,
    body: &VerifiedScalarBody,
    ids: &BTreeMap<String, LocalId>,
) -> Option<TypedStatement> {
    Some(match statement {
        ScalarStatement::Let { name, expr } => TypedStatement::Let {
            local: *ids.get(name)?,
            expr: lower_expr(expr, body, ids)?,
        },
        ScalarStatement::HostOutboundStatus { .. } => return None,
        ScalarStatement::Set { name, expr } => TypedStatement::Set {
            local: *ids.get(name)?,
            expr: lower_expr(expr, body, ids)?,
        },
        ScalarStatement::F32ArraySet {
            array,
            index,
            value,
        } => TypedStatement::ArraySet {
            array: *ids.get(array)?,
            index: lower_expr(index, body, ids)?,
            value: lower_expr(value, body, ids)?,
            proof: IndexProof::RuntimeChecked,
        },
        ScalarStatement::StringDictSet { .. } => return None,
        ScalarStatement::PureCall {
            target,
            function,
            args,
            array_lens,
            return_type,
            ..
        } => TypedStatement::PureCall {
            target: match target {
                Some(name) => Some(*ids.get(name)?),
                None => None,
            },
            function: function.clone(),
            args: args
                .iter()
                .map(|arg| match arg {
                    ScalarExpr::Variable(name) => ids.get(name).copied(),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?,
            array_lens: array_lens.clone(),
            return_type: match return_type {
                Some(ty) => Some(numeric_type(*ty)?),
                None => None,
            },
        },
        ScalarStatement::If {
            condition,
            statements,
        } => TypedStatement::If {
            condition: lower_expr(condition, body, ids)?,
            statements: statements
                .iter()
                .map(|s| lower_statement(s, body, ids))
                .collect::<Option<Vec<_>>>()?,
        },
        ScalarStatement::While {
            condition,
            statements,
        } => TypedStatement::While {
            condition: lower_expr(condition, body, ids)?,
            statements: statements
                .iter()
                .map(|s| lower_statement(s, body, ids))
                .collect::<Option<Vec<_>>>()?,
        },
        ScalarStatement::Return(expr) => TypedStatement::Return(lower_expr(expr, body, ids)?),
        ScalarStatement::ReturnHtml(parts) => TypedStatement::ReturnHtml(
            parts
                .iter()
                .map(|part| match part {
                    crate::NativeHtmlPart::Text(text) => Some(TypedHtmlPart::Text(text.clone())),
                    crate::NativeHtmlPart::Escaped(expr) => {
                        Some(TypedHtmlPart::Escaped(lower_expr(expr, body, ids)?))
                    }
                    crate::NativeHtmlPart::Safe(_) => None,
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        ScalarStatement::ReturnStruct { .. }
        | ScalarStatement::ReturnOption { .. }
        | ScalarStatement::ReturnResult { .. }
        | ScalarStatement::ReturnTypedJson { .. } => return None,
    })
}

fn lower_expr(
    expr: &ScalarExpr,
    body: &VerifiedScalarBody,
    ids: &BTreeMap<String, LocalId>,
) -> Option<TypedExpr> {
    let ty = scalar_expr_type(expr, body)?;
    let kind = match expr {
        ScalarExpr::Int(v) => TypedExprKind::Int(*v),
        ScalarExpr::F32(v) => TypedExprKind::F32(*v),
        ScalarExpr::Bool(v) => TypedExprKind::Bool(*v),
        ScalarExpr::Variable(name) => TypedExprKind::Local(*ids.get(name)?),
        ScalarExpr::Not(inner) => TypedExprKind::Not(Box::new(lower_expr(inner, body, ids)?)),
        ScalarExpr::F32ArrayNew { len, fill } => TypedExprKind::ArrayNew {
            element: NumericElementType::F32,
            len: Box::new(lower_expr(len, body, ids)?),
            fill: Box::new(lower_expr(fill, body, ids)?),
        },
        ScalarExpr::Binary { left, op, right } => TypedExprKind::Binary {
            left: Box::new(lower_expr(left, body, ids)?),
            op: *op,
            right: Box::new(lower_expr(right, body, ids)?),
        },
        ScalarExpr::Builtin { function, args } => TypedExprKind::Builtin {
            function: *function,
            args: args
                .iter()
                .map(|a| lower_expr(a, body, ids))
                .collect::<Option<Vec<_>>>()?,
        },
        ScalarExpr::CollectionLen { collection } => TypedExprKind::CollectionLen {
            collection: *ids.get(collection)?,
        },
        ScalarExpr::CollectionIndex { collection, index } => TypedExprKind::CollectionIndex {
            collection: *ids.get(collection)?,
            index: Box::new(lower_expr(index, body, ids)?),
            proof: IndexProof::RuntimeChecked,
        },
        ScalarExpr::Field { .. }
        | ScalarExpr::String(_)
        | ScalarExpr::SumPredicate { .. }
        | ScalarExpr::SumUnwrapOr { .. } => return None,
    };
    Some(TypedExpr { ty, kind })
}

fn scalar_expr_type(expr: &ScalarExpr, body: &VerifiedScalarBody) -> Option<NumericType> {
    Some(match expr {
        ScalarExpr::Int(_) => NumericType::I64,
        ScalarExpr::F32(_) => NumericType::F32,
        ScalarExpr::Bool(_) => NumericType::Bool,
        ScalarExpr::Variable(name) => numeric_type(*body.local_types.get(name)?)?,
        ScalarExpr::F32ArrayNew { .. } => NumericType::ARRAY_F32,
        ScalarExpr::CollectionLen { .. } => NumericType::I64,
        ScalarExpr::CollectionIndex { collection, .. } => {
            match numeric_type(*body.local_types.get(collection)?)? {
                NumericType::Collection(NumericCollectionType {
                    element: NumericElementType::F32,
                    ..
                }) => NumericType::F32,
                _ => return None,
            }
        }
        ScalarExpr::Not(_) => NumericType::Bool,
        ScalarExpr::Binary { left, op, .. } => match op {
            ScalarBinaryOp::LogicalAnd
            | ScalarBinaryOp::LogicalOr
            | ScalarBinaryOp::Lt
            | ScalarBinaryOp::Le
            | ScalarBinaryOp::Gt
            | ScalarBinaryOp::Ge
            | ScalarBinaryOp::Eq
            | ScalarBinaryOp::Ne => NumericType::Bool,
            ScalarBinaryOp::StringConcat => return None,
            _ => scalar_expr_type(left, body)?,
        },
        ScalarExpr::Builtin { function, .. } => match function {
            ScalarBuiltin::Sin
            | ScalarBuiltin::Cos
            | ScalarBuiltin::Sqrt
            | ScalarBuiltin::ToF32 => NumericType::F32,
            ScalarBuiltin::MonotonicNanos => NumericType::I64,
            _ => return None,
        },
        ScalarExpr::Field { .. }
        | ScalarExpr::String(_)
        | ScalarExpr::SumPredicate { .. }
        | ScalarExpr::SumUnwrapOr { .. } => return None,
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct RangeEnvValue {
    const_int: Option<i64>,
    array_len: Option<i64>,
}

fn analyze_ranges(statements: &mut [TypedStatement], locals: &[TypedLocal]) -> Vec<RangeProof> {
    let mut env = vec![RangeEnvValue::default(); locals.len()];
    for local in locals {
        if let NumericType::Collection(NumericCollectionType {
            kind: NumericCollectionKind::FixedArray(size),
            ..
        }) = local.ty
        {
            env[local.id.0 as usize].array_len = Some(i64::from(size));
        }
    }
    let mut proofs = Vec::new();
    analyze_sequence(statements, &mut env, &mut proofs);
    proofs
}

fn analyze_sequence(
    statements: &mut [TypedStatement],
    env: &mut [RangeEnvValue],
    proofs: &mut Vec<RangeProof>,
) {
    for statement in statements {
        match statement {
            TypedStatement::Let { local, expr } | TypedStatement::Set { local, expr } => {
                let value = infer_env_value(expr, env);
                env[local.0 as usize] = value;
                annotate_expr(expr, env, None, proofs);
            }
            TypedStatement::ArraySet {
                array,
                index,
                value,
                proof,
            } => {
                annotate_expr(index, env, None, proofs);
                annotate_expr(value, env, None, proofs);
                *proof = prove_static_index(*array, index, env, proofs);
            }
            TypedStatement::PureCall { target, .. } => {
                if let Some(local) = target {
                    env[local.0 as usize] = RangeEnvValue::default();
                }
            }
            TypedStatement::If {
                condition,
                statements,
            } => {
                annotate_expr(condition, env, None, proofs);
                let mut branch_env = env.to_vec();
                analyze_sequence(statements, &mut branch_env, proofs);
                // A one-sided branch cannot establish new facts after the branch.
            }
            TypedStatement::While {
                condition,
                statements,
            } => {
                annotate_expr(condition, env, None, proofs);
                let loop_range = recognize_loop(condition, statements, env);
                let mut body_env = env.to_vec();
                if let Some(range) = loop_range {
                    body_env[range.induction.0 as usize].const_int = None;
                    annotate_sequence_with_loop(statements, &mut body_env, range, proofs);
                } else {
                    analyze_sequence(statements, &mut body_env, proofs);
                }
                invalidate_written_locals(statements, env);
            }
            TypedStatement::Return(expr) => annotate_expr(expr, env, None, proofs),
            TypedStatement::ReturnHtml(parts) => {
                for part in parts {
                    if let TypedHtmlPart::Escaped(expr) = part {
                        annotate_expr(expr, env, None, proofs);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LoopRange {
    induction: LocalId,
    lower_inclusive: i64,
    upper_exclusive: i64,
    step: i64,
}

fn recognize_loop(
    condition: &TypedExpr,
    statements: &[TypedStatement],
    env: &[RangeEnvValue],
) -> Option<LoopRange> {
    let TypedExprKind::Binary {
        left,
        op: ScalarBinaryOp::Lt,
        right,
    } = &condition.kind
    else {
        return None;
    };
    let TypedExprKind::Local(induction) = &left.kind else {
        return None;
    };
    let induction = *induction;
    let lower = env[induction.0 as usize].const_int?;
    if lower < 0 {
        return None;
    }
    let upper = const_int_expr(right, env)?;
    if upper < lower {
        return None;
    }
    let step = find_positive_induction_step(statements, induction)?;
    Some(LoopRange {
        induction,
        lower_inclusive: lower,
        upper_exclusive: upper,
        step,
    })
}

fn find_positive_induction_step(statements: &[TypedStatement], induction: LocalId) -> Option<i64> {
    let mut found = None;
    for statement in statements {
        match statement {
            TypedStatement::Set { local, expr } if *local == induction => {
                let TypedExprKind::Binary {
                    left,
                    op: ScalarBinaryOp::Add,
                    right,
                } = &expr.kind
                else {
                    return None;
                };
                if !matches!(&left.kind, TypedExprKind::Local(id) if *id == induction) {
                    return None;
                }
                let TypedExprKind::Int(step) = &right.kind else {
                    return None;
                };
                let step = *step;
                if step <= 0 || found.replace(step).is_some() {
                    return None;
                }
            }
            TypedStatement::PureCall {
                target: Some(local),
                ..
            } if *local == induction => return None,
            TypedStatement::If { statements, .. } | TypedStatement::While { statements, .. } => {
                if writes_local(statements, induction) {
                    return None;
                }
            }
            _ => {}
        }
    }
    found
}

fn writes_local(statements: &[TypedStatement], target: LocalId) -> bool {
    statements.iter().any(|statement| match statement {
        TypedStatement::Set { local, .. } => *local == target,
        TypedStatement::PureCall {
            target: Some(local),
            ..
        } => *local == target,
        TypedStatement::If { statements, .. } | TypedStatement::While { statements, .. } => {
            writes_local(statements, target)
        }
        _ => false,
    })
}

fn annotate_sequence_with_loop(
    statements: &mut [TypedStatement],
    env: &mut [RangeEnvValue],
    range: LoopRange,
    proofs: &mut Vec<RangeProof>,
) {
    for statement in statements {
        match statement {
            TypedStatement::Let { local, expr } | TypedStatement::Set { local, expr } => {
                annotate_expr(expr, env, Some(range), proofs);
                env[local.0 as usize] = infer_env_value(expr, env);
                if *local == range.induction {
                    env[local.0 as usize].const_int = None;
                }
            }
            TypedStatement::ArraySet {
                array,
                index,
                value,
                proof,
            } => {
                annotate_expr(index, env, Some(range), proofs);
                annotate_expr(value, env, Some(range), proofs);
                *proof = prove_index(*array, index, env, range, proofs);
            }
            TypedStatement::PureCall { target, .. } => {
                if let Some(local) = target {
                    env[local.0 as usize] = RangeEnvValue::default();
                }
            }
            TypedStatement::If {
                condition,
                statements,
            } => {
                annotate_expr(condition, env, Some(range), proofs);
                let mut nested = env.to_vec();
                annotate_sequence_with_loop(statements, &mut nested, range, proofs);
            }
            TypedStatement::While {
                condition,
                statements,
            } => {
                annotate_expr(condition, env, Some(range), proofs);
                let mut nested = env.to_vec();
                analyze_sequence(statements, &mut nested, proofs);
                invalidate_written_locals(statements, env);
            }
            TypedStatement::Return(expr) => annotate_expr(expr, env, Some(range), proofs),
            TypedStatement::ReturnHtml(parts) => {
                for part in parts {
                    if let TypedHtmlPart::Escaped(expr) = part {
                        annotate_expr(expr, env, Some(range), proofs);
                    }
                }
            }
        }
    }
}

fn annotate_expr(
    expr: &mut TypedExpr,
    env: &[RangeEnvValue],
    loop_range: Option<LoopRange>,
    proofs: &mut Vec<RangeProof>,
) {
    match &mut expr.kind {
        TypedExprKind::Not(inner) => annotate_expr(inner, env, loop_range, proofs),
        TypedExprKind::ArrayNew { len, fill, .. } => {
            annotate_expr(len, env, loop_range, proofs);
            annotate_expr(fill, env, loop_range, proofs);
        }
        TypedExprKind::Binary { left, right, .. } => {
            annotate_expr(left, env, loop_range, proofs);
            annotate_expr(right, env, loop_range, proofs);
        }
        TypedExprKind::Builtin { args, .. } => {
            for arg in args {
                annotate_expr(arg, env, loop_range, proofs);
            }
        }
        TypedExprKind::CollectionIndex {
            collection,
            index,
            proof,
        } => {
            annotate_expr(index, env, loop_range, proofs);
            *proof = match loop_range {
                Some(range) => prove_index(*collection, index, env, range, proofs),
                None => prove_static_index(*collection, index, env, proofs),
            };
        }
        _ => {}
    }
}

fn prove_static_index(
    collection: LocalId,
    index: &TypedExpr,
    env: &[RangeEnvValue],
    proofs: &mut Vec<RangeProof>,
) -> IndexProof {
    let Some(array_len) = env[collection.0 as usize].array_len else {
        return IndexProof::RuntimeChecked;
    };
    let Some(value) = const_int_expr(index, env) else {
        return IndexProof::RuntimeChecked;
    };
    if value < 0 || value >= array_len {
        return IndexProof::RuntimeChecked;
    }
    let Some(upper) = value.checked_add(1) else {
        return IndexProof::RuntimeChecked;
    };
    record_range_proof(collection, LocalId(u32::MAX), value, upper, 1, proofs)
}

fn affine_induction_offset(
    index: &TypedExpr,
    env: &[RangeEnvValue],
    induction: LocalId,
) -> Option<i64> {
    match &index.kind {
        TypedExprKind::Local(id) if *id == induction => Some(0),
        TypedExprKind::Binary {
            left,
            op: ScalarBinaryOp::Add,
            right,
        } => {
            if matches!(&left.kind, TypedExprKind::Local(id) if *id == induction) {
                return const_int_expr(right, env);
            }
            if matches!(&right.kind, TypedExprKind::Local(id) if *id == induction) {
                return const_int_expr(left, env);
            }
            None
        }
        TypedExprKind::Binary {
            left,
            op: ScalarBinaryOp::Sub,
            right,
        } if matches!(&left.kind, TypedExprKind::Local(id) if *id == induction) => {
            const_int_expr(right, env)?.checked_neg()
        }
        _ => None,
    }
}

fn record_range_proof(
    collection: LocalId,
    index: LocalId,
    lower_inclusive: i64,
    upper_exclusive: i64,
    step: i64,
    proofs: &mut Vec<RangeProof>,
) -> IndexProof {
    let Ok(raw_id) = u32::try_from(proofs.len()) else {
        return IndexProof::RuntimeChecked;
    };
    let id = RangeProofId(raw_id);
    proofs.push(RangeProof {
        id,
        index,
        collection,
        lower_inclusive,
        upper_exclusive,
        step,
    });
    IndexProof::Proven(id)
}

fn prove_index(
    collection: LocalId,
    index: &TypedExpr,
    env: &[RangeEnvValue],
    range: LoopRange,
    proofs: &mut Vec<RangeProof>,
) -> IndexProof {
    let Some(array_len) = env[collection.0 as usize].array_len else {
        return IndexProof::RuntimeChecked;
    };
    let Some(offset) = affine_induction_offset(index, env, range.induction) else {
        return prove_static_index(collection, index, env, proofs);
    };
    let Some(lower) = range.lower_inclusive.checked_add(offset) else {
        return IndexProof::RuntimeChecked;
    };
    let Some(upper) = range.upper_exclusive.checked_add(offset) else {
        return IndexProof::RuntimeChecked;
    };
    if lower < 0 || upper > array_len {
        return IndexProof::RuntimeChecked;
    }
    let proof_index = if offset == 0 {
        range.induction
    } else {
        LocalId(u32::MAX)
    };
    record_range_proof(collection, proof_index, lower, upper, range.step, proofs)
}

fn infer_env_value(expr: &TypedExpr, env: &[RangeEnvValue]) -> RangeEnvValue {
    RangeEnvValue {
        const_int: const_int_expr(expr, env),
        array_len: match &expr.kind {
            TypedExprKind::ArrayNew { len, .. } => const_int_expr(len, env).filter(|v| *v >= 0),
            TypedExprKind::Local(local) => env[local.0 as usize].array_len,
            _ => None,
        },
    }
}

fn const_int_expr(expr: &TypedExpr, env: &[RangeEnvValue]) -> Option<i64> {
    match &expr.kind {
        TypedExprKind::Int(value) => Some(*value),
        TypedExprKind::Local(local) => env[local.0 as usize].const_int,
        TypedExprKind::CollectionLen { collection } => env[collection.0 as usize].array_len,
        TypedExprKind::Binary { left, op, right } => {
            let a = const_int_expr(left, env)?;
            let b = const_int_expr(right, env)?;
            match op {
                ScalarBinaryOp::Add => a.checked_add(b),
                ScalarBinaryOp::Sub => a.checked_sub(b),
                ScalarBinaryOp::Mul => a.checked_mul(b),
                ScalarBinaryOp::Div => a.checked_div(b),
                ScalarBinaryOp::Rem => a.checked_rem(b),
                _ => None,
            }
        }
        _ => None,
    }
}

fn invalidate_written_locals(statements: &[TypedStatement], env: &mut [RangeEnvValue]) {
    for statement in statements {
        match statement {
            TypedStatement::Set { local, .. } => env[local.0 as usize] = RangeEnvValue::default(),
            TypedStatement::PureCall { target, .. } => {
                if let Some(local) = target {
                    env[local.0 as usize] = RangeEnvValue::default();
                }
            }
            TypedStatement::If { statements, .. } | TypedStatement::While { statements, .. } => {
                invalidate_written_locals(statements, env)
            }
            _ => {}
        }
    }
}

#[derive(Default)]
struct CfgBuilder {
    nodes: Vec<CfgNode>,
}

impl CfgBuilder {
    fn push(&mut self, kind: CfgNodeKind) -> BlockId {
        let id = BlockId(self.nodes.len() as u32);
        self.nodes.push(CfgNode {
            id,
            kind,
            successors: Vec::new(),
        });
        id
    }
    fn edge(&mut self, from: BlockId, to: BlockId) {
        self.nodes[from.0 as usize].successors.push(to);
    }

    fn sequence(
        &mut self,
        statements: &[TypedStatement],
        continuation: Option<BlockId>,
    ) -> (Option<BlockId>, Vec<BlockId>) {
        let mut next = continuation;
        let mut tails = Vec::new();
        for statement in statements.iter().rev() {
            let (entry, statement_tails) = self.statement(statement, next);
            next = Some(entry);
            tails = statement_tails;
        }
        (next, tails)
    }

    fn statement(
        &mut self,
        statement: &TypedStatement,
        continuation: Option<BlockId>,
    ) -> (BlockId, Vec<BlockId>) {
        match statement {
            TypedStatement::Return(_) | TypedStatement::ReturnHtml(_) => {
                (self.push(CfgNodeKind::Return), Vec::new())
            }
            TypedStatement::If { statements, .. } => {
                let branch = self.push(CfgNodeKind::Branch);
                let (then_entry, _) = self.sequence(statements, continuation);
                if let Some(entry) = then_entry {
                    self.edge(branch, entry);
                }
                if let Some(cont) = continuation {
                    self.edge(branch, cont);
                }
                (branch, vec![branch])
            }
            TypedStatement::While { statements, .. } => {
                let condition = self.push(CfgNodeKind::LoopCondition);
                let (body_entry, _) = self.sequence(statements, Some(condition));
                if let Some(entry) = body_entry {
                    self.edge(condition, entry);
                } else {
                    self.edge(condition, condition);
                }
                if let Some(cont) = continuation {
                    self.edge(condition, cont);
                }
                (condition, vec![condition])
            }
            _ => {
                let node = self.push(CfgNodeKind::Operation);
                if let Some(cont) = continuation {
                    self.edge(node, cont);
                }
                (node, vec![node])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_access_is_explicitly_not_numeric_fast_path_eligible() {
        let body = VerifiedScalarBody::test_body(
            Vec::new(),
            Vec::new(),
            BTreeMap::from([("file".into(), NativeScalarType::Upload)]),
        );
        let ids = BTreeMap::new();
        let field = ScalarExpr::Field {
            base: "file".into(),
            field: "bytes".into(),
            ty: NativeScalarType::Int,
        };
        assert!(scalar_expr_type(&field, &body).is_none());
        assert!(lower_expr(&field, &body, &ids).is_none());
    }

    #[test]
    fn numeric_html_return_keeps_page_on_pure_typed_path() {
        let body = VerifiedScalarBody::test_body(
            Vec::new(),
            vec![
                ScalarStatement::Let {
                    name: "elapsed".into(),
                    expr: ScalarExpr::Int(42),
                },
                ScalarStatement::ReturnHtml(vec![
                    crate::NativeHtmlPart::Text("elapsed=".into()),
                    crate::NativeHtmlPart::Escaped(ScalarExpr::Variable("elapsed".into())),
                ]),
            ],
            BTreeMap::from([("elapsed".into(), NativeScalarType::Int)]),
        );
        let typed = lower(&body).expect("numeric page with numeric HTML interpolation stays typed");
        assert!(matches!(
            typed.statements().last(),
            Some(TypedStatement::ReturnHtml(_))
        ));
    }

    #[test]
    fn constant_numeric_arrays_promote_to_fixed_arrays() {
        let mut locals = vec![TypedLocal {
            id: LocalId(0),
            debug_name: "data".into(),
            ty: NumericType::ARRAY_F32,
            is_input: false,
        }];
        let mut statements = vec![TypedStatement::Let {
            local: LocalId(0),
            expr: TypedExpr {
                ty: NumericType::ARRAY_F32,
                kind: TypedExprKind::ArrayNew {
                    element: NumericElementType::F32,
                    len: Box::new(int(4096)),
                    fill: Box::new(TypedExpr {
                        ty: NumericType::F32,
                        kind: TypedExprKind::F32(0),
                    }),
                },
            },
        }];
        promote_constant_arrays(&mut statements, &mut locals);
        assert_eq!(
            locals[0].ty,
            NumericType::Collection(NumericCollectionType {
                element: NumericElementType::F32,
                kind: NumericCollectionKind::FixedArray(4096),
            })
        );
    }

    #[test]
    fn ids_are_dense_and_stable_for_inputs_then_lexical_lets() {
        let body = VerifiedScalarBody::test_numeric_body();
        let typed = lower(&body).expect("numeric lowering");
        assert_eq!(typed.locals()[0].debug_name, "input");
        assert_eq!(typed.locals()[1].debug_name, "x");
        assert_eq!(typed.locals()[0].id, LocalId(0));
        assert_eq!(typed.locals()[1].id, LocalId(1));
    }

    fn int(value: i64) -> TypedExpr {
        TypedExpr {
            ty: NumericType::I64,
            kind: TypedExprKind::Int(value),
        }
    }
    fn local(id: u32, ty: NumericType) -> TypedExpr {
        TypedExpr {
            ty,
            kind: TypedExprKind::Local(LocalId(id)),
        }
    }

    #[test]
    fn canonical_loop_produces_array_index_proofs() {
        let locals = vec![
            TypedLocal {
                id: LocalId(0),
                debug_name: "n".into(),
                ty: NumericType::I64,
                is_input: false,
            },
            TypedLocal {
                id: LocalId(1),
                debug_name: "data".into(),
                ty: NumericType::ARRAY_F32,
                is_input: false,
            },
            TypedLocal {
                id: LocalId(2),
                debug_name: "i".into(),
                ty: NumericType::I64,
                is_input: false,
            },
            TypedLocal {
                id: LocalId(3),
                debug_name: "x".into(),
                ty: NumericType::F32,
                is_input: false,
            },
        ];
        let mut statements = vec![
            TypedStatement::Let {
                local: LocalId(0),
                expr: int(4096),
            },
            TypedStatement::Let {
                local: LocalId(1),
                expr: TypedExpr {
                    ty: NumericType::ARRAY_F32,
                    kind: TypedExprKind::ArrayNew {
                        element: NumericElementType::F32,
                        len: Box::new(int(4096)),
                        fill: Box::new(TypedExpr {
                            ty: NumericType::F32,
                            kind: TypedExprKind::F32(0.0f32.to_bits()),
                        }),
                    },
                },
            },
            TypedStatement::Let {
                local: LocalId(2),
                expr: int(0),
            },
            TypedStatement::While {
                condition: TypedExpr {
                    ty: NumericType::Bool,
                    kind: TypedExprKind::Binary {
                        left: Box::new(local(2, NumericType::I64)),
                        op: ScalarBinaryOp::Lt,
                        right: Box::new(local(0, NumericType::I64)),
                    },
                },
                statements: vec![
                    TypedStatement::Let {
                        local: LocalId(3),
                        expr: TypedExpr {
                            ty: NumericType::F32,
                            kind: TypedExprKind::CollectionIndex {
                                collection: LocalId(1),
                                index: Box::new(local(2, NumericType::I64)),
                                proof: IndexProof::RuntimeChecked,
                            },
                        },
                    },
                    TypedStatement::ArraySet {
                        array: LocalId(1),
                        index: local(2, NumericType::I64),
                        value: local(3, NumericType::F32),
                        proof: IndexProof::RuntimeChecked,
                    },
                    TypedStatement::Set {
                        local: LocalId(2),
                        expr: TypedExpr {
                            ty: NumericType::I64,
                            kind: TypedExprKind::Binary {
                                left: Box::new(local(2, NumericType::I64)),
                                op: ScalarBinaryOp::Add,
                                right: Box::new(int(1)),
                            },
                        },
                    },
                ],
            },
        ];
        let proofs = analyze_ranges(&mut statements, &locals);
        assert_eq!(proofs.len(), 2);
        assert!(
            proofs
                .iter()
                .all(|p| p.lower_inclusive == 0 && p.upper_exclusive == 4096 && p.step == 1)
        );
        let TypedStatement::While {
            statements: body, ..
        } = &statements[3]
        else {
            panic!("while expected")
        };
        let TypedStatement::Let { expr, .. } = &body[0] else {
            panic!("let expected")
        };
        assert!(matches!(
            &expr.kind,
            TypedExprKind::CollectionIndex {
                proof: IndexProof::Proven(_),
                ..
            }
        ));
        let TypedStatement::ArraySet { proof, .. } = &body[1] else {
            panic!("array set expected")
        };
        assert!(matches!(proof, IndexProof::Proven(_)));
    }

    #[test]
    fn constant_index_is_proven_for_known_array_length() {
        let locals = vec![
            TypedLocal {
                id: LocalId(0),
                debug_name: "data".into(),
                ty: NumericType::ARRAY_F32,
                is_input: false,
            },
            TypedLocal {
                id: LocalId(1),
                debug_name: "x".into(),
                ty: NumericType::F32,
                is_input: false,
            },
        ];
        let mut statements = vec![
            TypedStatement::Let {
                local: LocalId(0),
                expr: TypedExpr {
                    ty: NumericType::ARRAY_F32,
                    kind: TypedExprKind::ArrayNew {
                        element: NumericElementType::F32,
                        len: Box::new(int(4096)),
                        fill: Box::new(TypedExpr {
                            ty: NumericType::F32,
                            kind: TypedExprKind::F32(0),
                        }),
                    },
                },
            },
            TypedStatement::Let {
                local: LocalId(1),
                expr: TypedExpr {
                    ty: NumericType::F32,
                    kind: TypedExprKind::CollectionIndex {
                        collection: LocalId(0),
                        index: Box::new(int(256)),
                        proof: IndexProof::RuntimeChecked,
                    },
                },
            },
        ];
        let proofs = analyze_ranges(&mut statements, &locals);
        assert_eq!(proofs.len(), 1);
        let TypedStatement::Let { expr, .. } = &statements[1] else {
            panic!("let expected")
        };
        assert!(matches!(
            &expr.kind,
            TypedExprKind::CollectionIndex {
                proof: IndexProof::Proven(_),
                ..
            }
        ));
    }

    #[test]
    fn affine_induction_plus_constant_is_proven_when_range_fits() {
        let locals = vec![
            TypedLocal {
                id: LocalId(0),
                debug_name: "data".into(),
                ty: NumericType::ARRAY_F32,
                is_input: false,
            },
            TypedLocal {
                id: LocalId(1),
                debug_name: "i".into(),
                ty: NumericType::I64,
                is_input: false,
            },
            TypedLocal {
                id: LocalId(2),
                debug_name: "x".into(),
                ty: NumericType::F32,
                is_input: false,
            },
        ];
        let index = TypedExpr {
            ty: NumericType::I64,
            kind: TypedExprKind::Binary {
                left: Box::new(local(1, NumericType::I64)),
                op: ScalarBinaryOp::Add,
                right: Box::new(int(1)),
            },
        };
        let mut statements = vec![
            TypedStatement::Let {
                local: LocalId(0),
                expr: TypedExpr {
                    ty: NumericType::ARRAY_F32,
                    kind: TypedExprKind::ArrayNew {
                        element: NumericElementType::F32,
                        len: Box::new(int(16)),
                        fill: Box::new(TypedExpr {
                            ty: NumericType::F32,
                            kind: TypedExprKind::F32(0),
                        }),
                    },
                },
            },
            TypedStatement::Let {
                local: LocalId(1),
                expr: int(0),
            },
            TypedStatement::While {
                condition: TypedExpr {
                    ty: NumericType::Bool,
                    kind: TypedExprKind::Binary {
                        left: Box::new(local(1, NumericType::I64)),
                        op: ScalarBinaryOp::Lt,
                        right: Box::new(int(15)),
                    },
                },
                statements: vec![
                    TypedStatement::Let {
                        local: LocalId(2),
                        expr: TypedExpr {
                            ty: NumericType::F32,
                            kind: TypedExprKind::CollectionIndex {
                                collection: LocalId(0),
                                index: Box::new(index),
                                proof: IndexProof::RuntimeChecked,
                            },
                        },
                    },
                    TypedStatement::Set {
                        local: LocalId(1),
                        expr: TypedExpr {
                            ty: NumericType::I64,
                            kind: TypedExprKind::Binary {
                                left: Box::new(local(1, NumericType::I64)),
                                op: ScalarBinaryOp::Add,
                                right: Box::new(int(1)),
                            },
                        },
                    },
                ],
            },
        ];
        let proofs = analyze_ranges(&mut statements, &locals);
        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0].lower_inclusive, 1);
        assert_eq!(proofs[0].upper_exclusive, 16);
        let TypedStatement::While {
            statements: body, ..
        } = &statements[2]
        else {
            panic!("while expected")
        };
        let TypedStatement::Let { expr, .. } = &body[0] else {
            panic!("let expected")
        };
        assert!(matches!(
            &expr.kind,
            TypedExprKind::CollectionIndex {
                proof: IndexProof::Proven(_),
                ..
            }
        ));
    }

    #[test]
    fn affine_index_stays_runtime_checked_when_shift_escapes_array() {
        let env = [RangeEnvValue {
            const_int: None,
            array_len: Some(16),
        }];
        let range = LoopRange {
            induction: LocalId(1),
            lower_inclusive: 0,
            upper_exclusive: 16,
            step: 1,
        };
        let index = TypedExpr {
            ty: NumericType::I64,
            kind: TypedExprKind::Binary {
                left: Box::new(local(1, NumericType::I64)),
                op: ScalarBinaryOp::Add,
                right: Box::new(int(1)),
            },
        };
        let mut proofs = Vec::new();
        assert_eq!(
            prove_index(LocalId(0), &index, &env, range, &mut proofs),
            IndexProof::RuntimeChecked
        );
        assert!(proofs.is_empty());
    }

    #[test]
    fn proof_is_not_emitted_when_loop_bound_exceeds_array() {
        let locals = vec![
            TypedLocal {
                id: LocalId(0),
                debug_name: "n".into(),
                ty: NumericType::I64,
                is_input: false,
            },
            TypedLocal {
                id: LocalId(1),
                debug_name: "data".into(),
                ty: NumericType::ARRAY_F32,
                is_input: false,
            },
            TypedLocal {
                id: LocalId(2),
                debug_name: "i".into(),
                ty: NumericType::I64,
                is_input: false,
            },
        ];
        let mut statements = vec![
            TypedStatement::Let {
                local: LocalId(0),
                expr: int(8),
            },
            TypedStatement::Let {
                local: LocalId(1),
                expr: TypedExpr {
                    ty: NumericType::ARRAY_F32,
                    kind: TypedExprKind::ArrayNew {
                        element: NumericElementType::F32,
                        len: Box::new(int(4)),
                        fill: Box::new(TypedExpr {
                            ty: NumericType::F32,
                            kind: TypedExprKind::F32(0),
                        }),
                    },
                },
            },
            TypedStatement::Let {
                local: LocalId(2),
                expr: int(0),
            },
            TypedStatement::While {
                condition: TypedExpr {
                    ty: NumericType::Bool,
                    kind: TypedExprKind::Binary {
                        left: Box::new(local(2, NumericType::I64)),
                        op: ScalarBinaryOp::Lt,
                        right: Box::new(local(0, NumericType::I64)),
                    },
                },
                statements: vec![
                    TypedStatement::ArraySet {
                        array: LocalId(1),
                        index: local(2, NumericType::I64),
                        value: TypedExpr {
                            ty: NumericType::F32,
                            kind: TypedExprKind::F32(0),
                        },
                        proof: IndexProof::RuntimeChecked,
                    },
                    TypedStatement::Set {
                        local: LocalId(2),
                        expr: TypedExpr {
                            ty: NumericType::I64,
                            kind: TypedExprKind::Binary {
                                left: Box::new(local(2, NumericType::I64)),
                                op: ScalarBinaryOp::Add,
                                right: Box::new(int(1)),
                            },
                        },
                    },
                ],
            },
        ];
        let proofs = analyze_ranges(&mut statements, &locals);
        assert!(proofs.is_empty());
        let TypedStatement::While {
            statements: body, ..
        } = &statements[3]
        else {
            panic!("while expected")
        };
        let TypedStatement::ArraySet { proof, .. } = &body[0] else {
            panic!("array set expected")
        };
        assert_eq!(*proof, IndexProof::RuntimeChecked);
    }
}

pub fn typed_statement_fuel(statement: &TypedStatement) -> u64 {
    match statement {
        TypedStatement::Let { expr, .. }
        | TypedStatement::Set { expr, .. }
        | TypedStatement::Return(expr) => 1 + typed_expr_fuel(expr),
        TypedStatement::ReturnHtml(parts) => {
            1 + parts
                .iter()
                .map(|part| match part {
                    TypedHtmlPart::Text(_) => 1,
                    TypedHtmlPart::Escaped(expr) => 1 + typed_expr_fuel(expr),
                })
                .sum::<u64>()
        }
        TypedStatement::ArraySet { index, value, .. } => {
            1 + typed_expr_fuel(index) + typed_expr_fuel(value)
        }
        TypedStatement::PureCall { .. } => 1,
        TypedStatement::If { condition, .. } | TypedStatement::While { condition, .. } => {
            1 + typed_expr_fuel(condition)
        }
    }
}

fn typed_expr_fuel(expr: &TypedExpr) -> u64 {
    1 + match &expr.kind {
        TypedExprKind::Int(_)
        | TypedExprKind::F32(_)
        | TypedExprKind::Bool(_)
        | TypedExprKind::Local(_)
        | TypedExprKind::CollectionLen { .. } => 0,
        TypedExprKind::ArrayNew { len, fill, .. } => {
            typed_expr_fuel(len).saturating_add(typed_expr_fuel(fill))
        }
        TypedExprKind::Not(inner) => typed_expr_fuel(inner),
        TypedExprKind::Binary { left, right, .. } => {
            typed_expr_fuel(left).saturating_add(typed_expr_fuel(right))
        }
        TypedExprKind::Builtin { function, args } => typed_function_fuel(*function)
            .saturating_add(args.iter().map(typed_expr_fuel).sum::<u64>()),
        TypedExprKind::CollectionIndex { index, .. } => typed_expr_fuel(index),
    }
}

fn typed_function_fuel(function: ScalarBuiltin) -> u64 {
    match function {
        ScalarBuiltin::Sin | ScalarBuiltin::Cos | ScalarBuiltin::Sqrt => 15,
        ScalarBuiltin::MonotonicNanos => 3,
        ScalarBuiltin::ToF32 => 1,
        _ => 1,
    }
}
