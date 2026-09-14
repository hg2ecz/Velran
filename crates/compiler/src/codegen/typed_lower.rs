use executable_ir::{
    IndexProof, LocalId, NumericCollectionKind, NumericElementType, NumericType, RangeProof,
    RangeProofId, ScalarBinaryOp, ScalarBuiltin, TypedExpr, TypedExprKind, TypedHtmlPart,
    TypedStatement, VerifiedNumericBody, typed_statement_fuel,
};

pub(crate) fn body(body: &VerifiedNumericBody) -> String {
    let mut out = String::new();
    statements_source(body, body.statements(), &mut out, 0, None);
    out
}

pub(crate) fn local_ident(id: LocalId) -> String {
    format!("velran_l{}", id.0)
}

fn is_batchable(statement: &TypedStatement) -> bool {
    matches!(
        statement,
        TypedStatement::Let { .. } | TypedStatement::Set { .. } | TypedStatement::ArraySet { .. }
    )
}

#[derive(Debug, Clone, Copy)]
struct CountedLoop {
    induction: LocalId,
    step: i64,
}

fn counted_straight_line_loop(
    condition: &TypedExpr,
    statements: &[TypedStatement],
) -> Option<CountedLoop> {
    let TypedExprKind::Binary {
        left,
        op: ScalarBinaryOp::Lt,
        ..
    } = &condition.kind
    else {
        return None;
    };
    let TypedExprKind::Local(induction) = &left.kind else {
        return None;
    };
    let induction = *induction;
    let mut step = None;
    for statement in statements {
        if !is_batchable(statement) {
            return None;
        }
        if let TypedStatement::Set { local, expr } = statement {
            if *local != induction {
                continue;
            }
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
            let TypedExprKind::Int(value) = &right.kind else {
                return None;
            };
            if *value <= 0 || step.replace(*value).is_some() {
                return None;
            }
        }
    }
    Some(CountedLoop {
        induction,
        step: step?,
    })
}

fn straight_line_fuel(statements: &[TypedStatement]) -> Option<u64> {
    let mut total = 0u64;
    for statement in statements {
        if !is_batchable(statement) {
            return None;
        }
        total = total.checked_add(typed_statement_fuel(statement))?;
    }
    Some(total)
}

fn counted_loop_preamble(
    body: &VerifiedNumericBody,
    condition: &TypedExpr,
    statements: &[TypedStatement],
    while_fuel: u64,
    pad: &str,
) -> Option<String> {
    let counted = counted_straight_line_loop(condition, statements)?;
    let TypedExprKind::Binary { right, .. } = &condition.kind else {
        return None;
    };
    let per_iteration = straight_line_fuel(statements)?.checked_add(while_fuel)?;
    let induction = local_ident(counted.induction);
    let upper = expr_value_source(body, right);
    let step = counted.step;
    Some(format!(
        "{pad}let velran_loop_upper = {upper};\n{pad}let velran_loop_remaining = if {induction} < velran_loop_upper {{ ((i128::from(velran_loop_upper) - i128::from({induction}) + i128::from({step}i64) - 1) / i128::from({step}i64)) as u128 }} else {{ 0u128 }};\n{pad}let velran_loop_fuel = velran_loop_remaining.checked_mul({per_iteration}u128).and_then(|v| u64::try_from(v).ok()).unwrap_or(u64::MAX);\n{pad}if !velran_fuel.charge(velran_loop_fuel) {{ return budget_exceeded(&velran_fuel, &velran_state); }}\n"
    ))
}

fn sin_cos_pair<'a>(
    first: &'a TypedStatement,
    second: &'a TypedStatement,
) -> Option<(LocalId, bool, LocalId, bool, &'a TypedExpr)> {
    fn parts(statement: &TypedStatement) -> Option<(LocalId, ScalarBuiltin, &TypedExpr)> {
        let TypedStatement::Let { local, expr } = statement else {
            return None;
        };
        let TypedExprKind::Builtin { function, args } = &expr.kind else {
            return None;
        };
        if args.len() != 1 || expr.ty != NumericType::F32 {
            return None;
        }
        if !matches!(function, ScalarBuiltin::Sin | ScalarBuiltin::Cos) {
            return None;
        }
        Some((*local, *function, &args[0]))
    }
    let (a_local, a_fn, a_arg) = parts(first)?;
    let (b_local, b_fn, b_arg) = parts(second)?;
    if a_arg != b_arg || a_fn == b_fn {
        return None;
    }
    Some((
        a_local,
        a_fn == ScalarBuiltin::Sin,
        b_local,
        b_fn == ScalarBuiltin::Sin,
        a_arg,
    ))
}

fn emit_sin_cos_pair(
    body: &VerifiedNumericBody,
    first: &TypedStatement,
    second: &TypedStatement,
    out: &mut String,
    indent: usize,
) -> bool {
    let Some((a_local, a_is_sin, b_local, b_is_sin, arg)) = sin_cos_pair(first, second) else {
        return false;
    };
    let pad = "    ".repeat(indent);
    let arg = expr_value_source(body, arg);
    let a_name = local_ident(a_local);
    let b_name = local_ident(b_local);
    let (sin_name, cos_name) = if a_is_sin {
        (a_name.as_str(), b_name.as_str())
    } else {
        (b_name.as_str(), a_name.as_str())
    };
    debug_assert_ne!(a_is_sin, b_is_sin);
    out.push_str(&format!("{pad}let velran_angle = {arg};\n"));
    out.push_str(&format!(
        "{pad}let (velran_sin, velran_cos) = velran_angle.sin_cos();\n"
    ));
    out.push_str(&format!("{pad}if !velran_sin.is_finite() || !velran_cos.is_finite() {{ return fail(&velran_fuel, &velran_state); }}\n"));
    out.push_str(&format!("{pad}let mut {sin_name}: f32 = velran_sin;\n"));
    out.push_str(&format!("{pad}let mut {cos_name}: f32 = velran_cos;\n"));
    true
}

fn statements_source(
    body: &VerifiedNumericBody,
    statements: &[TypedStatement],
    out: &mut String,
    indent: usize,
    loop_proof: Option<&RangeProof>,
) {
    let pad = "    ".repeat(indent);
    let mut index = 0usize;
    while index < statements.len() {
        if is_batchable(&statements[index]) {
            let start = index;
            let mut fuel = 0u64;
            while index < statements.len() && is_batchable(&statements[index]) {
                fuel = fuel.saturating_add(typed_statement_fuel(&statements[index]));
                index += 1;
            }
            out.push_str(&format!("{pad}if !velran_fuel.charge({fuel}) {{ return budget_exceeded(&velran_fuel, &velran_state); }}\n"));
            let mut batch_index = start;
            while batch_index < index {
                if batch_index + 1 < index
                    && emit_sin_cos_pair(
                        body,
                        &statements[batch_index],
                        &statements[batch_index + 1],
                        out,
                        indent,
                    )
                {
                    batch_index += 2;
                } else {
                    statement_source(
                        body,
                        &statements[batch_index],
                        out,
                        indent,
                        loop_proof,
                        false,
                    );
                    batch_index += 1;
                }
            }
        } else {
            statement_source(body, &statements[index], out, indent, loop_proof, true);
            index += 1;
        }
    }
}

fn statement_source(
    body: &VerifiedNumericBody,
    statement: &TypedStatement,
    out: &mut String,
    indent: usize,
    loop_proof: Option<&RangeProof>,
    emit_charge: bool,
) {
    let pad = "    ".repeat(indent);
    if emit_charge {
        out.push_str(&format!(
            "{pad}if !velran_fuel.charge({}) {{ return budget_exceeded(&velran_fuel, &velran_state); }}\n",
            typed_statement_fuel(statement)
        ));
    }
    match statement {
        TypedStatement::Let { local, expr } => {
            let ty = body.local(*local).ty;
            let value = match (ty, &expr.kind) {
                (NumericType::Collection(collection), TypedExprKind::ArrayNew { fill, .. })
                    if matches!(collection.kind, NumericCollectionKind::FixedArray(_)) =>
                {
                    let NumericCollectionKind::FixedArray(size) = collection.kind else {
                        unreachable!()
                    };
                    let bytes = u64::from(size) * 4;
                    format!(
                        "{{ if !velran_state.charge_alloc({bytes}) {{ return fail(&velran_fuel, &velran_state); }} [{}; {size}] }}",
                        expr_value_source(body, fill)
                    )
                }
                _ => expr_value_source(body, expr),
            };
            out.push_str(&format!(
                "{pad}let mut {}: {} = {value};\n",
                local_ident(*local),
                rust_type(ty)
            ));
        }
        TypedStatement::Set { local, expr } => {
            out.push_str(&format!(
                "{pad}{} = {};\n",
                local_ident(*local),
                expr_value_source(body, expr)
            ));
        }
        TypedStatement::PureCall {
            target,
            function,
            args,
            return_type,
            ..
        } => {
            let call_args = args
                .iter()
                .map(|arg| format!("&mut {}", local_ident(*arg)))
                .collect::<Vec<_>>()
                .join(", ");
            let comma = if call_args.is_empty() { "" } else { ", " };
            out.push_str(&format!("{pad}let velran_pure_call = {}({call_args}{comma}velran_fuel.remaining(), velran_state.remaining_alloc());\n", super::pure_function_ident(function)));
            out.push_str(&format!("{pad}if !velran_fuel.charge(velran_pure_call.3) {{ return budget_exceeded(&velran_fuel, &velran_state); }}\n"));
            out.push_str(&format!("{pad}if !velran_state.charge_alloc(velran_pure_call.4) {{ return fail(&velran_fuel, &velran_state); }}\n"));
            out.push_str(&format!("{pad}if velran_pure_call.0 != VELRAN_STATUS_OK {{ return (velran_pure_call.0, VELRAN_VALUE_NONE, 0, velran_fuel.used(), velran_state.allocated(), 0); }}\n"));
            if let (Some(local), Some(return_type)) = (target, return_type) {
                let (tag, decode) = match return_type {
                    NumericType::I64 => ("VELRAN_VALUE_INT", "velran_pure_call.2 as i64".to_string()),
                    NumericType::Bool => ("VELRAN_VALUE_BOOL", "{ if velran_pure_call.2 > 1 { velran_state.bad_request(); return fail(&velran_fuel, &velran_state); } velran_pure_call.2 != 0 }".to_string()),
                    NumericType::F32 => ("VELRAN_VALUE_F32_INTERNAL", "{ let Ok(velran_bits) = u32::try_from(velran_pure_call.2) else { velran_state.bad_request(); return fail(&velran_fuel, &velran_state); }; let velran_value = f32::from_bits(velran_bits); if !velran_value.is_finite() { velran_state.bad_request(); return fail(&velran_fuel, &velran_state); } velran_value }".to_string()),
                    NumericType::Collection(_) => unreachable!("pure scalar return contract"),
                };
                out.push_str(&format!("{pad}if velran_pure_call.1 != {tag} {{ velran_state.bad_request(); return fail(&velran_fuel, &velran_state); }}\n"));
                out.push_str(&format!(
                    "{pad}let mut {}: {} = {decode};\n",
                    local_ident(*local),
                    rust_type(*return_type)
                ));
            }
        }
        TypedStatement::ArraySet {
            array,
            index,
            value,
            proof,
        } => match proof {
            IndexProof::Proven(id) => {
                let range = body.range_proof(*id);
                debug_assert_eq!(range.collection, *array);
                out.push_str(&format!(
                    "{pad}let velran_index = {} as usize;\n",
                    expr_value_source(body, index)
                ));
                out.push_str(&format!(
                    "{pad}let velran_value = {};\n",
                    expr_value_source(body, value)
                ));
                out.push_str(&format!(
                    "{pad}{}[velran_index] = velran_value;\n",
                    local_ident(*array)
                ));
            }
            IndexProof::RuntimeChecked => {
                out.push_str(&format!(
                    "{pad}let velran_index_value = {};\n",
                    expr_value_source(body, index)
                ));
                out.push_str(&format!("{pad}let Ok(velran_index) = usize::try_from(velran_index_value) else {{ velran_state.bad_request(); return fail(&velran_fuel, &velran_state); }};\n"));
                out.push_str(&format!("{pad}if velran_index >= {}.len() {{ velran_state.bad_request(); return fail(&velran_fuel, &velran_state); }}\n", local_ident(*array)));
                out.push_str(&format!(
                    "{pad}let velran_value = {};\n",
                    expr_value_source(body, value)
                ));
                out.push_str(&format!(
                    "{pad}{}[velran_index] = velran_value;\n",
                    local_ident(*array)
                ));
            }
        },
        TypedStatement::If {
            condition,
            statements,
        } => {
            out.push_str(&format!(
                "{pad}if {} {{\n",
                expr_value_source(body, condition)
            ));
            statements_source(body, statements, out, indent + 1, loop_proof);
            out.push_str(&format!("{pad}}}\n"));
        }
        TypedStatement::While {
            condition,
            statements,
        } => {
            if let Some(preamble) = counted_loop_preamble(
                body,
                condition,
                statements,
                typed_statement_fuel(statement),
                &pad,
            ) {
                out.push_str(&preamble);
                out.push_str(&format!(
                    "{pad}while {} {{\n",
                    expr_value_source(body, condition)
                ));
                for nested in statements {
                    statement_source(body, nested, out, indent + 1, None, false);
                }
                out.push_str(&format!("{pad}}}\n"));
            } else if let Some(proof) = loop_proof_for(body, condition, statements) {
                out.push_str(&format!(
                    "{pad}while {} < {}i64 {{\n",
                    local_ident(proof.index),
                    proof.upper_exclusive
                ));
                statements_source(body, statements, out, indent + 1, Some(proof));
                out.push_str(&format!("{pad}    if !velran_fuel.charge({}) {{ return budget_exceeded(&velran_fuel, &velran_state); }}\n{pad}}}\n", typed_statement_fuel(statement)));
            } else {
                out.push_str(&format!(
                    "{pad}while {} {{\n",
                    expr_value_source(body, condition)
                ));
                statements_source(body, statements, out, indent + 1, None);
                out.push_str(&format!("{pad}    if !velran_fuel.charge({}) {{ return budget_exceeded(&velran_fuel, &velran_state); }}\n{pad}}}\n", typed_statement_fuel(statement)));
            }
        }
        TypedStatement::Return(expr) => {
            let finish = match expr.ty {
                NumericType::I64 => "typed_result_i64",
                NumericType::F32 => "typed_result_f32",
                NumericType::Bool => "typed_result_bool",
                NumericType::Collection(_) => "typed_result_unsupported",
            };
            out.push_str(&format!(
                "{pad}return {finish}({}, &velran_fuel, &velran_state);\n",
                expr_value_source(body, expr)
            ));
        }
        TypedStatement::ReturnHtml(parts) => {
            for part in parts {
                match part {
                    TypedHtmlPart::Text(text) => out.push_str(&format!("{pad}if !velran_output.push({text:?}, &mut velran_state) {{ return fail(&velran_fuel, &velran_state); }}\n")),
                    TypedHtmlPart::Escaped(expr) => {
                        let writer = match expr.ty {
                            NumericType::I64 => "typed_html_i64",
                            NumericType::F32 => "typed_html_f32",
                            NumericType::Bool => "typed_html_bool",
                            NumericType::Collection(_) => "typed_html_unsupported",
                        };
                        out.push_str(&format!("{pad}if !{writer}({}, &mut velran_output, &mut velran_state) {{ return fail(&velran_fuel, &velran_state); }}\n", expr_value_source(body, expr)));
                    }
                }
            }
            out.push_str(&format!(
                "{pad}return velran_output.finish(&velran_fuel, &velran_state);\n"
            ));
        }
    }
}

fn rust_type(ty: NumericType) -> String {
    match ty {
        NumericType::I64 => "i64".into(),
        NumericType::F32 => "f32".into(),
        NumericType::Bool => "bool".into(),
        NumericType::Collection(collection) => match (collection.element, collection.kind) {
            (NumericElementType::F32, NumericCollectionKind::Array) => "Vec<f32>".into(),
            (NumericElementType::F32, NumericCollectionKind::FixedArray(size)) => {
                format!("[f32; {size}]")
            }
            (NumericElementType::F32, NumericCollectionKind::SharedSlice) => {
                unreachable!("slice locals require lifetime-aware v2 frontend lowering")
            }
            (NumericElementType::F32, NumericCollectionKind::MutableSlice) => {
                unreachable!("slice locals require lifetime-aware v2 frontend lowering")
            }
        },
    }
}

fn expr_value_source(body: &VerifiedNumericBody, expr: &TypedExpr) -> String {
    if expr.ty == NumericType::F32 {
        return finite_f32_value(raw_f32_value_source(body, expr));
    }
    match &expr.kind {
        TypedExprKind::Int(value) => format!("{value}i64"),
        TypedExprKind::Bool(value) => format!("{value}"),
        TypedExprKind::Local(local) => local_ident(*local),
        TypedExprKind::ArrayNew {
            element: NumericElementType::F32,
            len,
            fill,
        } => format!(
            "match velran_numeric::typed_f32_array_new(Some({}), Some({}), &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            expr_value_source(body, len),
            expr_value_source(body, fill)
        ),
        TypedExprKind::CollectionLen { collection } => format!(
            "match i64::try_from({}.len()) {{ Ok(v) => v, Err(_) => return fail(&velran_fuel, &velran_state) }}",
            local_ident(*collection)
        ),
        TypedExprKind::Not(inner) => format!("!({})", expr_value_source(body, inner)),
        TypedExprKind::Builtin { function, args } => typed_builtin_value(body, *function, args),
        TypedExprKind::Binary { left, op, right } => typed_binary_value(body, left, *op, right),
        TypedExprKind::F32(_) | TypedExprKind::CollectionIndex { .. } => {
            unreachable!("f32 expressions are handled by raw_f32_value_source")
        }
    }
}

fn raw_f32_value_source(body: &VerifiedNumericBody, expr: &TypedExpr) -> String {
    debug_assert_eq!(expr.ty, NumericType::F32);
    match &expr.kind {
        TypedExprKind::F32(bits) => format!("f32::from_bits({bits}u32)"),
        TypedExprKind::Local(local) => local_ident(*local),
        TypedExprKind::CollectionIndex {
            collection,
            index,
            proof,
        } => match proof {
            IndexProof::Proven(id) => {
                let range = body.range_proof(*id);
                debug_assert_eq!(range.collection, *collection);
                format!(
                    "{}[({}) as usize]",
                    local_ident(*collection),
                    expr_value_source(body, index)
                )
            }
            IndexProof::RuntimeChecked => {
                let collection = local_ident(*collection);
                let index = expr_value_source(body, index);
                format!(
                    "{{ let velran_index_value = {index}; let Ok(velran_index) = usize::try_from(velran_index_value) else {{ velran_state.bad_request(); return fail(&velran_fuel, &velran_state); }}; if velran_index >= {collection}.len() {{ velran_state.bad_request(); return fail(&velran_fuel, &velran_state); }} {collection}[velran_index] }}"
                )
            }
        },
        TypedExprKind::Builtin { function, args } => {
            let a = |i: usize| {
                if args[i].ty == NumericType::F32 {
                    raw_f32_value_source(body, &args[i])
                } else {
                    expr_value_source(body, &args[i])
                }
            };
            match function {
                ScalarBuiltin::Sin => format!("({}).sin()", a(0)),
                ScalarBuiltin::Cos => format!("({}).cos()", a(0)),
                ScalarBuiltin::Sqrt => format!("({}).sqrt()", a(0)),
                ScalarBuiltin::ToF32 => format!("({}) as f32", expr_value_source(body, &args[0])),
                _ => unreachable!("typed numeric f32 path excludes non-numeric builtins"),
            }
        }
        TypedExprKind::Binary { left, op, right } => {
            use ScalarBinaryOp::*;
            let sym = match op {
                Add => "+",
                Sub => "-",
                Mul => "*",
                Div => "/",
                Rem => "%",
                _ => unreachable!("verified f32 expression"),
            };
            let l = if left.ty == NumericType::F32 {
                raw_f32_value_source(body, left)
            } else {
                expr_value_source(body, left)
            };
            let r = if right.ty == NumericType::F32 {
                raw_f32_value_source(body, right)
            } else {
                expr_value_source(body, right)
            };
            format!("({l}) {sym} ({r})")
        }
        _ => unreachable!("verified f32 expression kind"),
    }
}

fn finite_f32_value(expr: String) -> String {
    format!(
        "{{ let velran_v = {expr}; if !velran_v.is_finite() {{ return fail(&velran_fuel, &velran_state); }} velran_v }}"
    )
}

fn typed_builtin_value(
    body: &VerifiedNumericBody,
    function: ScalarBuiltin,
    args: &[TypedExpr],
) -> String {
    let a = |i: usize| expr_value_source(body, &args[i]);
    match function {
        ScalarBuiltin::Sin => finite_f32_value(format!("({}).sin()", a(0))),
        ScalarBuiltin::Cos => finite_f32_value(format!("({}).cos()", a(0))),
        ScalarBuiltin::Sqrt => {
            let value = a(0);
            format!(
                "{{ let velran_v = {value}; if velran_v < 0.0 {{ return fail(&velran_fuel, &velran_state); }} {} }}",
                finite_f32_value("velran_v.sqrt()".into())
            )
        }
        ScalarBuiltin::MonotonicNanos => "velran_numeric::typed_monotonic_nanos()".into(),
        ScalarBuiltin::ToF32 => finite_f32_value(format!("({}) as f32", a(0))),
        _ => unreachable!("typed numeric path excludes string builtins"),
    }
}

fn typed_binary_value(
    body: &VerifiedNumericBody,
    left: &TypedExpr,
    op: ScalarBinaryOp,
    right: &TypedExpr,
) -> String {
    let l = expr_value_source(body, left);
    let r = expr_value_source(body, right);
    use ScalarBinaryOp::*;
    match op {
        LogicalAnd => format!("({l}) && ({r})"),
        LogicalOr => format!("({l}) || ({r})"),
        Eq | Ne | Lt | Le | Gt | Ge => {
            let sym = match op {
                Eq => "==",
                Ne => "!=",
                Lt => "<",
                Le => "<=",
                Gt => ">",
                Ge => ">=",
                _ => unreachable!(),
            };
            format!("({l}) {sym} ({r})")
        }
        BitAnd | BitXor | BitOr => {
            let sym = match op {
                BitAnd => "&",
                BitXor => "^",
                BitOr => "|",
                _ => unreachable!(),
            };
            format!("({l}) {sym} ({r})")
        }
        Add | Sub | Mul | Div | Rem => match left.ty {
            NumericType::I64 => {
                let method = match op {
                    Add => "checked_add",
                    Sub => "checked_sub",
                    Mul => "checked_mul",
                    Div => "checked_div",
                    Rem => "checked_rem",
                    _ => unreachable!(),
                };
                format!(
                    "match ({l}).{method}({r}) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}"
                )
            }
            NumericType::F32 => {
                let sym = match op {
                    Add => "+",
                    Sub => "-",
                    Mul => "*",
                    Div => "/",
                    Rem => "%",
                    _ => unreachable!(),
                };
                finite_f32_value(format!("({l}) {sym} ({r})"))
            }
            _ => unreachable!("verified numeric operation"),
        },
        StringConcat => unreachable!("typed numeric path excludes strings"),
    }
}

fn loop_proof_for<'a>(
    body: &'a VerifiedNumericBody,
    condition: &TypedExpr,
    statements: &[TypedStatement],
) -> Option<&'a RangeProof> {
    let TypedExprKind::Binary {
        left,
        op: ScalarBinaryOp::Lt,
        ..
    } = &condition.kind
    else {
        return None;
    };
    let TypedExprKind::Local(induction) = &left.kind else {
        return None;
    };
    let mut ids = Vec::<RangeProofId>::new();
    collect_proof_ids(statements, &mut ids);
    ids.into_iter()
        .map(|id| body.range_proof(id))
        .find(|proof| proof.index == *induction)
}

fn collect_proof_ids(statements: &[TypedStatement], out: &mut Vec<RangeProofId>) {
    for statement in statements {
        match statement {
            TypedStatement::ArraySet {
                proof: IndexProof::Proven(id),
                index,
                value,
                ..
            } => {
                out.push(*id);
                collect_expr_proof_ids(index, out);
                collect_expr_proof_ids(value, out);
            }
            TypedStatement::ArraySet {
                index,
                value,
                proof: IndexProof::RuntimeChecked,
                ..
            } => {
                collect_expr_proof_ids(index, out);
                collect_expr_proof_ids(value, out);
            }
            TypedStatement::PureCall { .. } => {}
            TypedStatement::Let { expr, .. }
            | TypedStatement::Set { expr, .. }
            | TypedStatement::Return(expr) => collect_expr_proof_ids(expr, out),
            TypedStatement::ReturnHtml(parts) => {
                for part in parts {
                    if let TypedHtmlPart::Escaped(expr) = part {
                        collect_expr_proof_ids(expr, out);
                    }
                }
            }
            TypedStatement::If {
                condition,
                statements,
            }
            | TypedStatement::While {
                condition,
                statements,
            } => {
                collect_expr_proof_ids(condition, out);
                collect_proof_ids(statements, out);
            }
        }
    }
}

fn collect_expr_proof_ids(expr: &TypedExpr, out: &mut Vec<RangeProofId>) {
    match &expr.kind {
        TypedExprKind::CollectionIndex { index, proof, .. } => {
            if let IndexProof::Proven(id) = proof {
                out.push(*id);
            }
            collect_expr_proof_ids(index, out);
        }
        TypedExprKind::Not(inner) => collect_expr_proof_ids(inner, out),
        TypedExprKind::ArrayNew {
            element: NumericElementType::F32,
            len,
            fill,
        } => {
            collect_expr_proof_ids(len, out);
            collect_expr_proof_ids(fill, out);
        }
        TypedExprKind::Binary { left, right, .. } => {
            collect_expr_proof_ids(left, out);
            collect_expr_proof_ids(right, out);
        }
        TypedExprKind::Builtin { args, .. } => {
            for arg in args {
                collect_expr_proof_ids(arg, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rust_types_are_unboxed() {
        assert_eq!(rust_type(NumericType::F32), "f32");
        assert_eq!(rust_type(NumericType::ARRAY_F32), "Vec<f32>");
    }
    #[test]
    fn fixed_arrays_lower_to_native_rust_arrays() {
        assert_eq!(
            rust_type(NumericType::Collection(
                executable_ir::NumericCollectionType {
                    element: NumericElementType::F32,
                    kind: NumericCollectionKind::FixedArray(4096),
                }
            )),
            "[f32; 4096]"
        );
    }

    #[test]
    fn local_ids_generate_compact_identifiers() {
        assert_eq!(local_ident(LocalId(0)), "velran_l0");
        assert_eq!(local_ident(LocalId(17)), "velran_l17");
    }
}
