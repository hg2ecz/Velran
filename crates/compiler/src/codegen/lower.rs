use super::local_ident;
use executable_ir::{
    NativeHtmlPart, NativeJsonFieldType, NativeOutboundCall, NativePureValueType, ScalarBinaryOp,
    ScalarBuiltin, ScalarExpr, ScalarStatement, VerifiedScalarBody, statement_fuel,
};

pub(crate) fn body(body: &VerifiedScalarBody) -> String {
    body_with_mode(body, LowerMode::Handler)
}

pub(crate) fn pure_body(body: &VerifiedScalarBody) -> String {
    body_with_mode(body, LowerMode::PureScalar)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LowerMode {
    Handler,
    PureScalar,
}

fn body_with_mode(body: &VerifiedScalarBody, mode: LowerMode) -> String {
    let mut out = String::new();
    statements_source(body.statements(), &mut out, 0, mode);
    out
}

fn helper<'a>(mode: LowerMode, handler: &'a str, pure: &'a str) -> &'a str {
    if mode == LowerMode::Handler {
        handler
    } else {
        pure
    }
}

fn statements_source(
    statements: &[ScalarStatement],
    out: &mut String,
    indent: usize,
    mode: LowerMode,
) {
    for statement in statements {
        statement_source(statement, out, indent, mode);
    }
}

fn statement_source(statement: &ScalarStatement, out: &mut String, indent: usize, mode: LowerMode) {
    let pad = "    ".repeat(indent);
    let fail_fn = helper(mode, "fail", "pure_fail");
    let budget_fn = helper(mode, "budget_exceeded", "pure_budget_exceeded");
    let result_fn = helper(mode, "result", "pure_result");
    out.push_str(&format!(
        "{pad}if !velran_fuel.charge({}) {{ return {budget_fn}(&velran_fuel, &velran_state); }}\n",
        statement_fuel(statement)
    ));
    match statement {
        ScalarStatement::Let { name, expr } => out.push_str(&format!(
            "{pad}let mut {} = match {} {{ Some(v) => v, None => return {fail_fn}(&velran_fuel, &velran_state) }};\n", local_ident(name), expr_source(expr))),
        ScalarStatement::Set { name, expr } => out.push_str(&format!(
            "{pad}{} = match {} {{ Some(v) => v, None => return {fail_fn}(&velran_fuel, &velran_state) }};\n", local_ident(name), expr_source(expr))),
        ScalarStatement::HostOutboundStatus { name, call } => {
            out.push_str(&format!("{pad}let {} = match {} {{ Some(v) => Scalar::Int(v), None => return {fail_fn}(&velran_fuel, &velran_state) }};\n", local_ident(name), outbound_source(call)));
        }
        ScalarStatement::PureCall { target, function, args, param_types, return_type, .. } => {
            let scalar_transport = !param_types
                .iter()
                .any(|ty| matches!(ty, executable_ir::NativeInputType::F32Array));
            let mut call_args = Vec::with_capacity(args.len());
            for (arg, param_ty) in args.iter().zip(param_types) {
                match param_ty {
                    executable_ir::NativeInputType::Int => call_args.push(format!(
                        "match {} {{ Some(Scalar::Int(v)) => v, _ => return {fail_fn}(&velran_fuel, &velran_state) }}", expr_source(arg)
                    )),
                    executable_ir::NativeInputType::Bool => call_args.push(format!(
                        "match {} {{ Some(Scalar::Bool(v)) => v, _ => return {fail_fn}(&velran_fuel, &velran_state) }}", expr_source(arg)
                    )),
                    executable_ir::NativeInputType::String => call_args.push(format!(
                        "match {} {{ Some(Scalar::String(v)) => v, _ => return {fail_fn}(&velran_fuel, &velran_state) }}", expr_source(arg)
                    )),
                    executable_ir::NativeInputType::StringList => call_args.push(format!(
                        "match {} {{ Some(Scalar::StringList(v)) => v, _ => return {fail_fn}(&velran_fuel, &velran_state) }}", expr_source(arg)
                    )),
                    executable_ir::NativeInputType::Struct(id) => call_args.push(format!(
                        "match {} {{ Some(Scalar::Struct(v @ PureStructValue::S{id} {{ .. }})) => v, _ => return {fail_fn}(&velran_fuel, &velran_state) }}", expr_source(arg)
                    )),
                    executable_ir::NativeInputType::F32Array => unreachable!("generic scalar pure-call codegen excludes numeric array helpers"),
                    _ => unreachable!("verified pure-call parameter type"),
                }
            }
            let joined = call_args.join(", ");
            let comma = if joined.is_empty() { "" } else { ", " };
            if scalar_transport {
                let recursion_arg = if mode == LowerMode::PureScalar { "velran_recursion_budget.saturating_sub(1)" } else { "VELRAN_MAX_PURE_CALL_DEPTH" };
                out.push_str(&format!("{pad}let velran_pure_call = {}({joined}{comma}velran_fuel.remaining(), velran_state.remaining_alloc(), {recursion_arg});\n", super::pure_function_ident(function)));
                out.push_str(&format!("{pad}if !velran_fuel.charge(velran_pure_call.fuel_used) {{ return {budget_fn}(&velran_fuel, &velran_state); }}\n"));
                out.push_str(&format!("{pad}if !velran_state.charge_alloc(velran_pure_call.allocated) {{ return {fail_fn}(&velran_fuel, &velran_state); }}\n"));
                if mode == LowerMode::PureScalar {
                    out.push_str(&format!("{pad}if velran_pure_call.status != VELRAN_STATUS_OK {{ return PureScalarResult {{ status: velran_pure_call.status, value: None, fuel_used: velran_fuel.used(), allocated: velran_state.allocated() }}; }}\n"));
                } else {
                    out.push_str(&format!("{pad}if velran_pure_call.status != VELRAN_STATUS_OK {{ return (velran_pure_call.status, VELRAN_VALUE_NONE, 0, velran_fuel.used(), velran_state.allocated(), 0); }}\n"));
                }
                if let (Some(target), Some(return_type)) = (target, return_type) {
                    let expected = match return_type {
                        executable_ir::NativeScalarType::Int => "Scalar::Int(_) => true".to_string(),
                        executable_ir::NativeScalarType::Bool => "Scalar::Bool(_) => true".to_string(),
                        executable_ir::NativeScalarType::F32 => "Scalar::F32(v) => v.is_finite()".to_string(),
                        executable_ir::NativeScalarType::String | executable_ir::NativeScalarType::SafeHtml => "Scalar::String(_) => true".to_string(),
                        executable_ir::NativeScalarType::StringList => "Scalar::StringList(_) => true".to_string(),
                        executable_ir::NativeScalarType::Struct(id) => format!("Scalar::Struct(PureStructValue::S{id} {{ .. }}) => true"),
                        executable_ir::NativeScalarType::Option(inner) => {
                            let suffix = sum_type_suffix(*inner);
                            format!("Scalar::PureSum(PureSumValue::Option{suffix}Some(_) | PureSumValue::Option{suffix}None) => true")
                        }
                        executable_ir::NativeScalarType::Result { ok, err } => {
                            let ok_s = sum_type_suffix(*ok); let err_s = sum_type_suffix(*err);
                            format!("Scalar::PureSum(PureSumValue::Result{ok_s}{err_s}Ok(_) | PureSumValue::Result{ok_s}{err_s}Err(_)) => true")
                        }
                        _ => unreachable!("pure scalar return contract"),
                    };
                    out.push_str(&format!("{pad}let mut {} = match velran_pure_call.value {{ Some(v) if match &v {{ {expected}, _ => false }} => v, _ => {{ velran_state.bad_request(); return {fail_fn}(&velran_fuel, &velran_state); }} }};\n", local_ident(target)));
                }
            } else {
                // Numeric helpers retain the primitive tuple transport used by the typed hot path.
                out.push_str(&format!("{pad}let velran_pure_call = {}({joined}{comma}velran_fuel.remaining(), velran_state.remaining_alloc());\n", super::pure_function_ident(function)));
                out.push_str(&format!("{pad}if !velran_fuel.charge(velran_pure_call.3) {{ return {budget_fn}(&velran_fuel, &velran_state); }}\n"));
                out.push_str(&format!("{pad}if !velran_state.charge_alloc(velran_pure_call.4) {{ return {fail_fn}(&velran_fuel, &velran_state); }}\n"));
                if mode == LowerMode::PureScalar {
                    out.push_str(&format!("{pad}if velran_pure_call.0 != VELRAN_STATUS_OK {{ return PureScalarResult {{ status: velran_pure_call.0, value: None, fuel_used: velran_fuel.used(), allocated: velran_state.allocated() }}; }}\n"));
                } else {
                    out.push_str(&format!("{pad}if velran_pure_call.0 != VELRAN_STATUS_OK {{ return (velran_pure_call.0, VELRAN_VALUE_NONE, 0, velran_fuel.used(), velran_state.allocated(), 0); }}\n"));
                }
                if let (Some(target), Some(return_type)) = (target, return_type) {
                    let decode = match return_type {
                        executable_ir::NativeScalarType::Int => "if velran_pure_call.1 != VELRAN_VALUE_INT { velran_state.bad_request(); return {fail_fn}(&velran_fuel, &velran_state); } Scalar::Int(velran_pure_call.2 as i64)".to_string(),
                        executable_ir::NativeScalarType::Bool => "if velran_pure_call.1 != VELRAN_VALUE_BOOL || velran_pure_call.2 > 1 { velran_state.bad_request(); return {fail_fn}(&velran_fuel, &velran_state); } Scalar::Bool(velran_pure_call.2 != 0)".to_string(),
                        executable_ir::NativeScalarType::F32 => "if velran_pure_call.1 != VELRAN_VALUE_F32_INTERNAL { velran_state.bad_request(); return {fail_fn}(&velran_fuel, &velran_state); } { let Ok(bits) = u32::try_from(velran_pure_call.2) else { velran_state.bad_request(); return {fail_fn}(&velran_fuel, &velran_state); }; let value = f32::from_bits(bits); if !value.is_finite() { velran_state.bad_request(); return {fail_fn}(&velran_fuel, &velran_state); } Scalar::F32(value) }".to_string(),
                        executable_ir::NativeScalarType::String | executable_ir::NativeScalarType::SafeHtml | executable_ir::NativeScalarType::StringList => unreachable!("owned scalar returns use scalar transport"),
                        _ => unreachable!("pure numeric return contract"),
                    };
                    out.push_str(&format!("{pad}let mut {} = {{ {decode} }};\n", local_ident(target)));
                }
            }
        },
        ScalarStatement::F32ArraySet { array, index, value } => {
            out.push_str(&format!("{pad}{{\n"));
            out.push_str(&format!("{pad}    let velran_array_index = {};\n", expr_source(index)));
            out.push_str(&format!("{pad}    let velran_array_value = {};\n", expr_source(value)));
            out.push_str(&format!(
                "{pad}    if !velran_numeric::f32_array_set(&mut {}, velran_array_index, velran_array_value, &mut velran_state) {{ return {fail_fn}(&velran_fuel, &velran_state); }}\n",
                local_ident(array),
            ));
            out.push_str(&format!("{pad}}}\n"));
        }
        ScalarStatement::StringDictSet { dict, key, value } => {
            out.push_str(&format!("{pad}{{\n"));
            out.push_str(&format!("{pad}    let velran_dict_key = {};\n", expr_source(key)));
            out.push_str(&format!("{pad}    let velran_dict_value = {};\n", expr_source(value)));
            out.push_str(&format!(
                "{pad}    if !string_dict_set(&mut {}, velran_dict_key, velran_dict_value, &mut velran_state) {{ return {fail_fn}(&velran_fuel, &velran_state); }}\n",
                local_ident(dict),
            ));
            out.push_str(&format!("{pad}}}\n"));
        }
        ScalarStatement::If { condition, statements } => {
            out.push_str(&format!("{pad}match {}.and_then(as_bool) {{\n", expr_source(condition)));
            out.push_str(&format!("{pad}    Some(true) => {{\n"));
            statements_source(statements, out, indent + 2, mode);
            out.push_str(&format!("{pad}    }}\n{pad}    Some(false) => {{}},\n{pad}    None => return {fail_fn}(&velran_fuel, &velran_state),\n{pad}}}\n"));
        }
        ScalarStatement::While { condition, statements } => {
            out.push_str(&format!("{pad}loop {{\n"));
            out.push_str(&format!("{pad}    match {}.and_then(as_bool) {{\n", expr_source(condition)));
            out.push_str(&format!("{pad}        Some(true) => {{}},\n{pad}        Some(false) => break,\n{pad}        None => return {fail_fn}(&velran_fuel, &velran_state),\n{pad}    }}\n"));
            statements_source(statements, out, indent + 1, mode);
            out.push_str(&format!("{pad}    if !velran_fuel.charge({}) {{ return {budget_fn}(&velran_fuel, &velran_state); }}\n", statement_fuel(statement)));
            out.push_str(&format!("{pad}}}\n"));
        }
        ScalarStatement::ReturnOption { inner_type, value } => {
            let suffix = sum_type_suffix(*inner_type);
            match value {
                Some(value) => {
                    let value_expr = expr_source(value);
                    let extract = sum_extract_expr(*inner_type, "velran_sum_scalar", fail_fn);
                    out.push_str(&format!("{pad}let velran_sum_scalar = match {value_expr} {{ Some(v) => v, None => return {fail_fn}(&velran_fuel, &velran_state) }};\n"));
                    out.push_str(&format!("{pad}let velran_sum_value = {{ {extract} }};\n"));
                    out.push_str(&format!("{pad}return {result_fn}(Scalar::PureSum(PureSumValue::Option{suffix}Some(velran_sum_value)), &velran_fuel, &velran_state);\n"));
                }
                None => out.push_str(&format!("{pad}return {result_fn}(Scalar::PureSum(PureSumValue::Option{suffix}None), &velran_fuel, &velran_state);\n")),
            }
        }
        ScalarStatement::ReturnResult { ok_type, err_type, is_ok, value } => {
            let ok_s = sum_type_suffix(*ok_type); let err_s = sum_type_suffix(*err_type);
            let payload_ty = if *is_ok { *ok_type } else { *err_type };
            let value_expr = expr_source(value);
            let extract = sum_extract_expr(payload_ty, "velran_sum_scalar", fail_fn);
            let variant = if *is_ok { "Ok" } else { "Err" };
            out.push_str(&format!("{pad}let velran_sum_scalar = match {value_expr} {{ Some(v) => v, None => return {fail_fn}(&velran_fuel, &velran_state) }};\n"));
            out.push_str(&format!("{pad}let velran_sum_value = {{ {extract} }};\n"));
            out.push_str(&format!("{pad}return {result_fn}(Scalar::PureSum(PureSumValue::Result{ok_s}{err_s}{variant}(velran_sum_value)), &velran_fuel, &velran_state);\n"));
        }
        ScalarStatement::ReturnStruct { struct_id, fields } => {
            let mut values = Vec::with_capacity(fields.len());
            for (index, field) in fields.iter().enumerate() {
                let expr = expr_source(&field.expr);
                let decode = match field.ty {
                    NativeJsonFieldType::Int => format!("match {expr} {{ Some(Scalar::Int(v)) => v, _ => return {fail_fn}(&velran_fuel, &velran_state) }}"),
                    NativeJsonFieldType::F32 => format!("match {expr} {{ Some(Scalar::F32(v)) if v.is_finite() => v, _ => return {fail_fn}(&velran_fuel, &velran_state) }}"),
                    NativeJsonFieldType::Bool => format!("match {expr} {{ Some(Scalar::Bool(v)) => v, _ => return {fail_fn}(&velran_fuel, &velran_state) }}"),
                    NativeJsonFieldType::String => format!("match {expr} {{ Some(Scalar::String(v)) => v, _ => return {fail_fn}(&velran_fuel, &velran_state) }}"),
                    NativeJsonFieldType::StringList => format!("match {expr} {{ Some(Scalar::StringList(v)) => v, _ => return {fail_fn}(&velran_fuel, &velran_state) }}"),
                };
                values.push(format!("f{index}: {decode}"));
            }
            out.push_str(&format!("{pad}return {result_fn}(Scalar::Struct(PureStructValue::S{struct_id} {{ {} }}), &velran_fuel, &velran_state);\n", values.join(", ")));
        }
        ScalarStatement::Return(expr) => out.push_str(&format!(
            "{pad}return match {} {{ Some(v) => {result_fn}(v, &velran_fuel, &velran_state), None => {fail_fn}(&velran_fuel, &velran_state) }};\n", expr_source(expr))),
        ScalarStatement::ReturnHtml(parts) => {
            for part in parts {
                match part {
                    NativeHtmlPart::Text(text) => out.push_str(&format!("{pad}if !velran_output.push({text:?}, &mut velran_state) {{ return {fail_fn}(&velran_fuel, &velran_state); }}\n")),
                    NativeHtmlPart::Escaped(expr) => out.push_str(&format!("{pad}match {} {{ Some(v) => if html_escape(v, &mut velran_output, &mut velran_state).is_none() {{ return {fail_fn}(&velran_fuel, &velran_state); }}, None => return {fail_fn}(&velran_fuel, &velran_state) }};\n", expr_source(expr))),
                    NativeHtmlPart::Safe(expr) => out.push_str(&format!("{pad}match {} {{ Some(Scalar::String(v)) => if !velran_output.push(v.as_ref(), &mut velran_state) {{ return {fail_fn}(&velran_fuel, &velran_state); }}, _ => return {fail_fn}(&velran_fuel, &velran_state) }};\n", expr_source(expr))),
                }
            }
            out.push_str(&format!("{pad}return velran_output.finish(&velran_fuel, &velran_state);\n"));
        }
        ScalarStatement::ReturnTypedJson { schema: _, fields } => {
            out.push_str(&format!("{pad}if !typed_json_begin(&mut velran_output, {}, &mut velran_state) {{ return {fail_fn}(&velran_fuel, &velran_state); }}\n", fields.len()));
            for field in fields {
                let tag = match field.ty {
                    NativeJsonFieldType::Int => 1u8, NativeJsonFieldType::Bool => 2u8, NativeJsonFieldType::F32 => 3u8,
                    NativeJsonFieldType::String => 4u8, NativeJsonFieldType::StringList => 5u8,
                };
                out.push_str(&format!("{pad}match {} {{ Some(v) => if !typed_json_field(&mut velran_output, {:?}, {tag}u8, v, &mut velran_state) {{ return {fail_fn}(&velran_fuel, &velran_state); }}, None => return {fail_fn}(&velran_fuel, &velran_state) }};\n", expr_source(&field.expr), field.name));
            }
            out.push_str(&format!("{pad}return velran_output.finish_typed_json(&velran_fuel, &velran_state);\n"));
        }
    }
}

fn outbound_source(call: &NativeOutboundCall) -> String {
    let method = if call.post_json { "1u8" } else { "0u8" };
    let body = match &call.body {
        Some(expr) => format!("scalar_json_body({}, &mut velran_state)", expr_source(expr)),
        None => "Some(None)".into(),
    };
    format!(
        "match {body} {{ Some(body) => velran_host.outbound_status({method}, {:?}, {:?}, body.as_deref(), &mut velran_state), None => None }}",
        call.target, call.path
    )
}

fn sum_type_suffix(ty: NativePureValueType) -> &'static str {
    match ty {
        NativePureValueType::Int => "Int",
        NativePureValueType::F32 => "F32",
        NativePureValueType::Bool => "Bool",
        NativePureValueType::String => "String",
        NativePureValueType::StringList => "StringList",
        NativePureValueType::SafeHtml => "String",
        NativePureValueType::Struct(_) => {
            unreachable!("sum struct payload lowering is not enabled")
        }
    }
}

fn sum_extract_expr(ty: NativePureValueType, variable: &str, fail_fn: &str) -> String {
    let pattern = match ty {
        NativePureValueType::Int => "Scalar::Int(v) => v",
        NativePureValueType::F32 => "Scalar::F32(v) if v.is_finite() => v",
        NativePureValueType::Bool => "Scalar::Bool(v) => v",
        NativePureValueType::String => "Scalar::String(v) => v",
        NativePureValueType::StringList => "Scalar::StringList(v) => v",
        NativePureValueType::SafeHtml => "Scalar::String(v) => v",
        NativePureValueType::Struct(_) => {
            unreachable!("sum struct payload lowering is not enabled")
        }
    };
    format!(
        "match {variable} {{ {pattern}, _ => {{ velran_state.bad_request(); return {fail_fn}(&velran_fuel, &velran_state); }} }}"
    )
}

fn expr_source(expr: &ScalarExpr) -> String {
    match expr {
        ScalarExpr::String(value) => format!("alloc_string({value:?}, &mut velran_state)"),
        ScalarExpr::Int(value) => format!("Some(Scalar::Int({value}i64))"),
        ScalarExpr::F32(bits) => format!("Some(Scalar::F32(f32::from_bits({bits}u32)))"),
        ScalarExpr::F32ArrayNew { len, fill } => format!(
            "velran_numeric::f32_array_new({}, {}, &mut velran_state)",
            expr_source(len),
            expr_source(fill)
        ),
        ScalarExpr::Bool(value) => format!("Some(Scalar::Bool({value}))"),
        ScalarExpr::Variable(name) => format!("Some({}.clone())", local_ident(name)),
        ScalarExpr::CollectionLen { collection } => {
            format!("collection_len(&{})", local_ident(collection))
        }
        ScalarExpr::CollectionIndex { collection, index } => format!(
            "collection_index(&{}, {}, &mut velran_state)",
            local_ident(collection),
            expr_source(index)
        ),
        ScalarExpr::Field { base, field, .. } => format!(
            "field_value(&{}, {:?}, &mut velran_state)",
            local_ident(base),
            field
        ),
        ScalarExpr::SumPredicate {
            base,
            predicate,
            base_type,
        } => sum_predicate_source(base, *predicate, *base_type),
        ScalarExpr::SumUnwrapOr {
            base,
            fallback,
            base_type,
        } => sum_unwrap_or_source(base, fallback, *base_type),
        ScalarExpr::Builtin { function, args } => builtin_source(*function, args),
        ScalarExpr::Not(inner) => format!(
            "{}.and_then(as_bool).map(|v| Scalar::Bool(!v))",
            expr_source(inner)
        ),
        ScalarExpr::Binary { left, op, right } => binary_source(left, *op, right),
    }
}

fn pure_value_variant_suffix(ty: NativePureValueType) -> &'static str {
    match ty {
        NativePureValueType::Int => "Int",
        NativePureValueType::F32 => "F32",
        NativePureValueType::Bool => "Bool",
        NativePureValueType::String => "String",
        NativePureValueType::StringList => "StringList",
        NativePureValueType::SafeHtml => "String",
        NativePureValueType::Struct(_) => {
            unreachable!("sum struct payload lowering is not enabled")
        }
    }
}

fn sum_payload_to_scalar(ty: NativePureValueType, value: &str) -> String {
    match ty {
        NativePureValueType::Int => format!("Scalar::Int({value})"),
        NativePureValueType::F32 => format!("Scalar::F32({value})"),
        NativePureValueType::Bool => format!("Scalar::Bool({value})"),
        NativePureValueType::String => format!("Scalar::String({value}.clone())"),
        NativePureValueType::StringList => format!("Scalar::StringList({value}.clone())"),
        NativePureValueType::SafeHtml => format!("Scalar::String({value}.clone())"),
        NativePureValueType::Struct(_) => {
            unreachable!("sum struct payload lowering is not enabled")
        }
    }
}

fn sum_predicate_source(
    base: &str,
    predicate: language_core::PureSumPredicate,
    base_type: executable_ir::NativeScalarType,
) -> String {
    use language_core::PureSumPredicate::*;
    let local = local_ident(base);
    let pattern = match (base_type, predicate) {
        (executable_ir::NativeScalarType::Option(inner), IsSome) => format!(
            "PureSumValue::Option{}Some(_)",
            pure_value_variant_suffix(inner)
        ),
        (executable_ir::NativeScalarType::Option(inner), IsNone) => format!(
            "PureSumValue::Option{}None",
            pure_value_variant_suffix(inner)
        ),
        (executable_ir::NativeScalarType::Result { ok, err }, IsOk) => format!(
            "PureSumValue::Result{}{}Ok(_)",
            pure_value_variant_suffix(ok),
            pure_value_variant_suffix(err)
        ),
        (executable_ir::NativeScalarType::Result { ok, err }, IsErr) => format!(
            "PureSumValue::Result{}{}Err(_)",
            pure_value_variant_suffix(ok),
            pure_value_variant_suffix(err)
        ),
        _ => unreachable!("verified sum predicate contract"),
    };
    format!("Some(Scalar::Bool(matches!(&{local}, Scalar::PureSum({pattern}))))")
}

fn sum_unwrap_or_source(
    base: &str,
    fallback: &ScalarExpr,
    base_type: executable_ir::NativeScalarType,
) -> String {
    let local = local_ident(base);
    let fallback = expr_source(fallback);
    let (pattern, payload_ty) = match base_type {
        executable_ir::NativeScalarType::Option(inner) => (
            format!(
                "PureSumValue::Option{}Some(v)",
                pure_value_variant_suffix(inner)
            ),
            inner,
        ),
        executable_ir::NativeScalarType::Result { ok, err } => (
            format!(
                "PureSumValue::Result{}{}Ok(v)",
                pure_value_variant_suffix(ok),
                pure_value_variant_suffix(err)
            ),
            ok,
        ),
        _ => unreachable!("verified unwrap_or contract"),
    };
    let scalar = sum_payload_to_scalar(payload_ty, "v");
    format!(
        "match &{local} {{ Scalar::PureSum({pattern}) => Some({scalar}), Scalar::PureSum(_) => {fallback}, _ => None }}"
    )
}

fn builtin_source(function: ScalarBuiltin, args: &[ScalarExpr]) -> String {
    use ScalarBuiltin::*;
    let a = |i| expr_source(&args[i]);
    match function {
        Sin => format!("velran_numeric::f32_unary({}, f32::sin)", a(0)),
        Cos => format!("velran_numeric::f32_unary({}, f32::cos)", a(0)),
        Sqrt => format!("velran_numeric::f32_sqrt({})", a(0)),
        MonotonicNanos => "velran_numeric::monotonic_nanos()".into(),
        ToF32 => format!("velran_numeric::to_f32({})", a(0)),
        StringLen => format!("string_len({})", a(0)),
        Trim => format!("string_trim({}, 0, &mut velran_state)", a(0)),
        TrimStart => format!("string_trim({}, 1, &mut velran_state)", a(0)),
        TrimEnd => format!("string_trim({}, 2, &mut velran_state)", a(0)),
        Lower => format!("string_case({}, false, &mut velran_state)", a(0)),
        Upper => format!("string_case({}, true, &mut velran_state)", a(0)),
        Contains => format!("string_predicate({}, {}, |a,b| a.contains(b))", a(0), a(1)),
        StartsWith => format!(
            "string_predicate({}, {}, |a,b| a.starts_with(b))",
            a(0),
            a(1)
        ),
        EndsWith => format!("string_predicate({}, {}, |a,b| a.ends_with(b))", a(0), a(1)),
        Replace => format!(
            "string_replace({}, {}, {}, &mut velran_state)",
            a(0),
            a(1),
            a(2)
        ),
        SplitBounded => format!(
            "split_bounded({}, {}, {}, &mut velran_state)",
            a(0),
            a(1),
            a(2)
        ),
        Substring => {
            if args.len() == 2 {
                format!("substring({}, {}, None, &mut velran_state)", a(0), a(1))
            } else {
                format!(
                    "substring({}, {}, Some({}), &mut velran_state)",
                    a(0),
                    a(1),
                    a(2)
                )
            }
        }
        IndexOf => format!("index_of({}, {}, false)", a(0), a(1)),
        LastIndexOf => format!("index_of({}, {}, true)", a(0), a(1)),
        CharAt => format!("char_at({}, {}, &mut velran_state)", a(0), a(1)),
        Repeat => format!("repeat_string({}, {}, &mut velran_state)", a(0), a(1)),
        SafeHtmlEmpty => "safe_html_empty()".into(),
        SafeHtmlText => format!("safe_html_text({}, &mut velran_state)", a(0)),
        SafeHtmlElement => format!("safe_html_element({}, {}, &mut velran_state)", a(0), a(1)),
        SafeHtmlLink => format!("safe_html_link({}, {}, &mut velran_state)", a(0), a(1)),
        SafeHtmlConcat => format!("safe_html_concat({}, {}, &mut velran_state)", a(0), a(1)),
        DictNew => "string_dict_new()".into(),
        ContainsKey => format!("string_dict_contains_key({}, {})", a(0), a(1)),
        RemoveKey => format!(
            "string_dict_remove_key({}, {}, &mut velran_state)",
            a(0),
            a(1)
        ),
    }
}

fn binary_source(left: &ScalarExpr, op: ScalarBinaryOp, right: &ScalarExpr) -> String {
    let left = expr_source(left);
    if matches!(op, ScalarBinaryOp::LogicalAnd | ScalarBinaryOp::LogicalOr) {
        let right = expr_source(right);
        return match op {
            ScalarBinaryOp::LogicalAnd => format!(
                "match {left}.and_then(as_bool) {{ Some(false) => Some(Scalar::Bool(false)), Some(true) => {right}.and_then(as_bool).map(Scalar::Bool), None => None }}"
            ),
            ScalarBinaryOp::LogicalOr => format!(
                "match {left}.and_then(as_bool) {{ Some(true) => Some(Scalar::Bool(true)), Some(false) => {right}.and_then(as_bool).map(Scalar::Bool), None => None }}"
            ),
            _ => unreachable!(),
        };
    }
    let pair = format!("({left}, {})", expr_source(right));
    match op {
        ScalarBinaryOp::Add => numeric(pair, "velran_numeric::numeric_add"),
        ScalarBinaryOp::Sub => numeric(pair, "velran_numeric::numeric_sub"),
        ScalarBinaryOp::Mul => numeric(pair, "velran_numeric::numeric_mul"),
        ScalarBinaryOp::Div => numeric(pair, "velran_numeric::numeric_div"),
        ScalarBinaryOp::Rem => numeric(pair, "velran_numeric::numeric_rem"),
        ScalarBinaryOp::BitAnd => int_op(pair, "|a,b| a & b"),
        ScalarBinaryOp::BitXor => int_op(pair, "|a,b| a ^ b"),
        ScalarBinaryOp::BitOr => int_op(pair, "|a,b| a | b"),
        ScalarBinaryOp::Lt => compare(pair, "velran_numeric::numeric_lt"),
        ScalarBinaryOp::Le => compare(pair, "velran_numeric::numeric_le"),
        ScalarBinaryOp::Gt => compare(pair, "velran_numeric::numeric_gt"),
        ScalarBinaryOp::Ge => compare(pair, "velran_numeric::numeric_ge"),
        ScalarBinaryOp::Eq => equality(pair, false),
        ScalarBinaryOp::Ne => equality(pair, true),
        ScalarBinaryOp::StringConcat => format!("concat_strings({pair}, &mut velran_state)"),
        ScalarBinaryOp::LogicalAnd | ScalarBinaryOp::LogicalOr => unreachable!(),
    }
}

fn numeric(pair: String, function: &str) -> String {
    format!("match {pair} {{ (Some(a), Some(b)) => {function}(a,b), _ => None }}")
}
fn int_op(pair: String, op: &str) -> String {
    format!(
        "match {pair} {{ (Some(a), Some(b)) => match (as_int(a), as_int(b)) {{ (Some(a), Some(b)) => Some(Scalar::Int(({op})(a,b))), _ => None }}, _ => None }}"
    )
}
fn compare(pair: String, function: &str) -> String {
    format!("{function}{pair}")
}
fn equality(pair: String, negate: bool) -> String {
    let n = if negate { "!" } else { "" };
    format!(
        "match {pair} {{ (Some(Scalar::Int(a)), Some(Scalar::Int(b))) => Some(Scalar::Bool({n}(a == b))), (Some(Scalar::F32(a)), Some(Scalar::F32(b))) => Some(Scalar::Bool({n}(a == b))), (Some(Scalar::Bool(a)), Some(Scalar::Bool(b))) => Some(Scalar::Bool({n}(a == b))), (Some(Scalar::String(a)), Some(Scalar::String(b))) => Some(Scalar::Bool({n}(a == b))), _ => None }}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn string_literal_charges_allocation() {
        assert!(expr_source(&ScalarExpr::String("abc".into())).contains("alloc_string"));
    }
    #[test]
    fn bounded_split_uses_runtime_allocation_state() {
        let e = ScalarExpr::Builtin {
            function: ScalarBuiltin::SplitBounded,
            args: vec![
                ScalarExpr::String("a,b".into()),
                ScalarExpr::String(",".into()),
                ScalarExpr::Int(2),
            ],
        };
        assert!(expr_source(&e).contains("split_bounded"));
    }
}

#[cfg(test)]
mod native_f32_codegen_tests {
    use super::*;

    #[test]
    fn f32_array_set_is_in_place() {
        let statement = ScalarStatement::F32ArraySet {
            array: "real".into(),
            index: ScalarExpr::Int(1),
            value: ScalarExpr::F32(0),
        };
        let mut out = String::new();
        statement_source(&statement, &mut out, 0, LowerMode::Handler);
        assert!(out.contains("f32_array_set(&mut velran_local_real"));
    }

    #[test]
    fn f32_math_uses_native_helpers() {
        let expr = ScalarExpr::Builtin {
            function: ScalarBuiltin::Sin,
            args: vec![ScalarExpr::F32(0)],
        };
        assert!(expr_source(&expr).contains("f32_unary"));
    }
}
