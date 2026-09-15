use super::local_ident;
use executable_ir::{
    NativeHtmlPart, NativeInputType, NativePureValueType, NativeScalarType, ScalarBinaryOp,
    ScalarBuiltin, ScalarExpr, ScalarStatement, VerifiedScalarBody, statement_fuel,
};

pub(crate) fn eligible(body: &VerifiedScalarBody) -> bool {
    if body.uses_host_api() || body.numeric_body().is_some() {
        return false;
    }
    if body.local_types().any(|ty| {
        !matches!(
            ty,
            NativeScalarType::Int
                | NativeScalarType::Bool
                | NativeScalarType::String
                | NativeScalarType::StringList
                | NativeScalarType::StringDict
        )
    }) {
        return false;
    }
    body.inputs().iter().all(|input| {
        matches!(
            input.ty,
            NativeInputType::Int
                | NativeInputType::Bool
                | NativeInputType::String
                | NativeInputType::Email
                | NativeInputType::Url
                | NativeInputType::Slug
                | NativeInputType::DomainInt { .. }
                | NativeInputType::DomainBool { .. }
                | NativeInputType::DomainString { .. }
        )
    }) && statements_supported(body.statements())
}

fn statements_supported(statements: &[ScalarStatement]) -> bool {
    statements.iter().all(|statement| match statement {
        ScalarStatement::Let { expr, .. }
        | ScalarStatement::Set { expr, .. }
        | ScalarStatement::Return(expr) => expr_supported(expr),
        ScalarStatement::StringDictSet { key, value, .. } => {
            expr_supported(key) && expr_supported(value)
        }
        ScalarStatement::If {
            condition,
            statements,
        }
        | ScalarStatement::While {
            condition,
            statements,
        } => expr_supported(condition) && statements_supported(statements),
        ScalarStatement::ReturnHtml(parts) => parts.iter().all(|part| match part {
            NativeHtmlPart::Text(_) => true,
            NativeHtmlPart::Escaped(expr) | NativeHtmlPart::Safe(expr) => expr_supported(expr),
        }),
        ScalarStatement::HostOutboundStatus { .. }
        | ScalarStatement::F32ArraySet { .. }
        | ScalarStatement::PureCall { .. }
        | ScalarStatement::ReturnStruct { .. }
        | ScalarStatement::ReturnOption { .. }
        | ScalarStatement::ReturnResult { .. }
        | ScalarStatement::ReturnTypedJson { .. } => false,
    })
}

fn expr_supported(expr: &ScalarExpr) -> bool {
    match expr {
        ScalarExpr::String(_)
        | ScalarExpr::Int(_)
        | ScalarExpr::Bool(_)
        | ScalarExpr::Variable(_)
        | ScalarExpr::CollectionLen { .. } => true,
        ScalarExpr::Not(inner) => expr_supported(inner),
        ScalarExpr::Binary { left, right, .. } => expr_supported(left) && expr_supported(right),
        ScalarExpr::Builtin { function, args } => {
            !matches!(
                function,
                ScalarBuiltin::Sin
                    | ScalarBuiltin::Cos
                    | ScalarBuiltin::Sqrt
                    | ScalarBuiltin::MonotonicNanos
                    | ScalarBuiltin::ToF32
            ) && args.iter().all(expr_supported)
        }
        ScalarExpr::CollectionIndex { index, .. } => expr_supported(index),
        ScalarExpr::F32(_)
        | ScalarExpr::F32ArrayNew { .. }
        | ScalarExpr::Field { .. }
        | ScalarExpr::SumPredicate { .. }
        | ScalarExpr::SumUnwrapOr { .. } => false,
    }
}

pub(crate) fn body(body: &VerifiedScalarBody) -> String {
    let mut out = String::new();
    statements_source(body, body.statements(), &mut out, 0);
    out
}

pub(crate) fn input_binder(ty: &NativeInputType, index: usize) -> String {
    match ty {
        NativeInputType::Int => format!("direct_input_int(&inputs[{index}], &mut velran_state)"),
        NativeInputType::Bool => format!("direct_input_bool(&inputs[{index}], &mut velran_state)"),
        NativeInputType::String => {
            format!("direct_input_string(&inputs[{index}], 0, &[], &mut velran_state)")
        }
        NativeInputType::Email => {
            format!("direct_input_string(&inputs[{index}], 1, &[], &mut velran_state)")
        }
        NativeInputType::Url => {
            format!("direct_input_string(&inputs[{index}], 2, &[], &mut velran_state)")
        }
        NativeInputType::Slug => {
            format!("direct_input_string(&inputs[{index}], 3, &[], &mut velran_state)")
        }
        NativeInputType::DomainInt { domain, ranges } => {
            let checks = ranges
                .iter()
                .map(|(min, max)| format!("({min}i64,{max}i64)"))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "direct_input_domain_int(&inputs[{index}], {domain}u16, &[{checks}], &mut velran_state)"
            )
        }
        NativeInputType::DomainBool { domain } => {
            format!("direct_input_domain_bool(&inputs[{index}], {domain}u16, &mut velran_state)")
        }
        NativeInputType::DomainString { domain, lengths } => {
            let checks = lengths
                .iter()
                .map(|(min, max)| format!("({min}usize,{max}usize)"))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "direct_input_domain_string(&inputs[{index}], {domain}u16, &[{checks}], &mut velran_state)"
            )
        }
        NativeInputType::F32Array
        | NativeInputType::StringList
        | NativeInputType::Struct(_)
        | NativeInputType::Upload
        | NativeInputType::Image => {
            unreachable!("typed scalar eligibility excludes internal collections/upload/image")
        }
    }
}

pub(crate) fn rust_type(ty: NativeScalarType) -> &'static str {
    match ty {
        NativeScalarType::Int => "i64",
        NativeScalarType::Bool => "bool",
        NativeScalarType::String | NativeScalarType::SafeHtml => "std::sync::Arc<str>",
        NativeScalarType::StringList => "std::sync::Arc<Vec<std::sync::Arc<str>>>",
        NativeScalarType::StringDict => {
            "std::sync::Arc<std::collections::BTreeMap<std::sync::Arc<str>, std::sync::Arc<str>>>"
        }
        _ => unreachable!("typed scalar eligibility excludes this type"),
    }
}

fn statements_source(
    body: &VerifiedScalarBody,
    statements: &[ScalarStatement],
    out: &mut String,
    indent: usize,
) {
    for statement in statements {
        statement_source(body, statement, out, indent);
    }
}

fn statement_source(
    body: &VerifiedScalarBody,
    statement: &ScalarStatement,
    out: &mut String,
    indent: usize,
) {
    let pad = "    ".repeat(indent);
    out.push_str(&format!(
        "{pad}if !velran_fuel.charge({}) {{ return budget_exceeded(&velran_fuel, &velran_state); }}\n",
        statement_fuel(statement)
    ));
    match statement {
        ScalarStatement::Let { name, expr } => {
            let ty = body.local_type(name).expect("verified local type");
            out.push_str(&format!("{pad}let mut {}: {} = {};\n", local_ident(name), rust_type(ty), expr_source(body, expr)));
        }
        ScalarStatement::Set { name, expr } => out.push_str(&format!("{pad}{} = {};\n", local_ident(name), expr_source(body, expr))),
        ScalarStatement::StringDictSet { dict, key, value } => out.push_str(&format!(
            "{pad}if !direct_dict_set(&mut {}, {}, {}, &mut velran_state) {{ return fail(&velran_fuel, &velran_state); }}\n",
            local_ident(dict), expr_source(body, key), expr_source(body, value))),
        ScalarStatement::If { condition, statements } => {
            out.push_str(&format!("{pad}if {} {{\n", expr_source(body, condition)));
            statements_source(body, statements, out, indent + 1);
            out.push_str(&format!("{pad}}}\n"));
        }
        ScalarStatement::While { condition, statements } => {
            out.push_str(&format!("{pad}while {} {{\n", expr_source(body, condition)));
            statements_source(body, statements, out, indent + 1);
            out.push_str(&format!("{pad}    if !velran_fuel.charge({}) {{ return budget_exceeded(&velran_fuel, &velran_state); }}\n", statement_fuel(statement)));
            out.push_str(&format!("{pad}}}\n"));
        }
        ScalarStatement::Return(expr) => {
            let ty = expr_type(body, expr);
            let result = match ty { NativeScalarType::Int => "typed_result_i64", NativeScalarType::Bool => "typed_result_bool", _ => "typed_result_unsupported" };
            out.push_str(&format!("{pad}return {result}({}, &velran_fuel, &velran_state);\n", expr_source(body, expr)));
        }
        ScalarStatement::ReturnHtml(parts) => {
            for part in parts {
                match part {
                    NativeHtmlPart::Text(text) => out.push_str(&format!("{pad}if !velran_output.push({text:?}, &mut velran_state) {{ return fail(&velran_fuel, &velran_state); }}\n")),
                    NativeHtmlPart::Escaped(expr) => {
                        let (function, value) = match expr_type(body, expr) {
                            NativeScalarType::Int => ("typed_html_i64", expr_source(body, expr)),
                            NativeScalarType::Bool => ("typed_html_bool", expr_source(body, expr)),
                            NativeScalarType::String => ("direct_html_string", borrowed_expr_source(body, expr)),
                            _ => ("typed_html_unsupported", expr_source(body, expr)),
                        };
                        out.push_str(&format!("{pad}if !{function}({value}, &mut velran_output, &mut velran_state) {{ return fail(&velran_fuel, &velran_state); }}\n"));
                    }
                    NativeHtmlPart::Safe(expr) => {
                        debug_assert_eq!(expr_type(body, expr), NativeScalarType::SafeHtml);
                        let value = borrowed_expr_source(body, expr);
                        out.push_str(&format!("{pad}if !velran_output.push({value}.as_ref(), &mut velran_state) {{ return fail(&velran_fuel, &velran_state); }}\n"));
                    }
                }
            }
            out.push_str(&format!("{pad}return velran_output.finish(&velran_fuel, &velran_state);\n"));
        }
        ScalarStatement::HostOutboundStatus { .. } | ScalarStatement::F32ArraySet { .. } | ScalarStatement::PureCall { .. } | ScalarStatement::ReturnStruct { .. } | ScalarStatement::ReturnOption { .. } | ScalarStatement::ReturnResult { .. } | ScalarStatement::ReturnTypedJson { .. } => unreachable!("typed scalar eligibility excludes this statement"),
    }
}

fn expr_source(body: &VerifiedScalarBody, expr: &ScalarExpr) -> String {
    match expr {
        ScalarExpr::String(value) => format!(
            "match direct_alloc_string({value:?}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}"
        ),
        ScalarExpr::Int(value) => format!("{value}i64"),
        ScalarExpr::Bool(value) => value.to_string(),
        ScalarExpr::Variable(name) => {
            match body.local_type(name).expect("verified variable type") {
                NativeScalarType::Int | NativeScalarType::Bool => local_ident(name),
                _ => format!("{}.clone()", local_ident(name)),
            }
        }
        ScalarExpr::CollectionLen { collection } => match body
            .local_type(collection)
            .expect("verified collection type")
        {
            NativeScalarType::String => format!("direct_string_len(&{})", local_ident(collection)),
            NativeScalarType::StringList | NativeScalarType::StringDict => format!(
                "i64::try_from({}.len()).unwrap_or(i64::MAX)",
                local_ident(collection)
            ),
            _ => unreachable!("unsupported direct collection"),
        },
        ScalarExpr::CollectionIndex { collection, index } => match body
            .local_type(collection)
            .expect("verified collection type")
        {
            NativeScalarType::StringList => format!(
                "match direct_list_index(&{}, {}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
                local_ident(collection),
                expr_source(body, index)
            ),
            NativeScalarType::StringDict => format!(
                "match direct_dict_index(&{}, {}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
                local_ident(collection),
                borrowed_expr_source(body, index)
            ),
            _ => unreachable!("unsupported direct collection index"),
        },
        ScalarExpr::Builtin { function, args } => builtin_source(body, *function, args),
        ScalarExpr::Not(inner) => format!("!({})", expr_source(body, inner)),
        ScalarExpr::Binary { left, op, right } => binary_source(body, left, *op, right),
        ScalarExpr::F32(_)
        | ScalarExpr::F32ArrayNew { .. }
        | ScalarExpr::Field { .. }
        | ScalarExpr::SumPredicate { .. }
        | ScalarExpr::SumUnwrapOr { .. } => {
            unreachable!("typed scalar eligibility excludes this expression")
        }
    }
}

fn borrowed_expr_source(body: &VerifiedScalarBody, expr: &ScalarExpr) -> String {
    match expr {
        ScalarExpr::Variable(name) => format!("&{}", local_ident(name)),
        _ => format!("&({})", expr_source(body, expr)),
    }
}

fn builtin_source(
    body: &VerifiedScalarBody,
    function: ScalarBuiltin,
    args: &[ScalarExpr],
) -> String {
    let a = |i: usize| expr_source(body, &args[i]);
    let b = |i: usize| borrowed_expr_source(body, &args[i]);
    match function {
        ScalarBuiltin::StringLen => format!("direct_string_len({})", b(0)),
        ScalarBuiltin::Trim => format!(
            "match direct_string_trim({}, 0, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0)
        ),
        ScalarBuiltin::TrimStart => format!(
            "match direct_string_trim({}, 1, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0)
        ),
        ScalarBuiltin::TrimEnd => format!(
            "match direct_string_trim({}, 2, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0)
        ),
        ScalarBuiltin::Lower => format!(
            "match direct_string_case({}, false, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0)
        ),
        ScalarBuiltin::Upper => format!(
            "match direct_string_case({}, true, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0)
        ),
        ScalarBuiltin::Contains => format!("({}).contains(({}).as_ref())", b(0), b(1)),
        ScalarBuiltin::StartsWith => format!("({}).starts_with(({}).as_ref())", b(0), b(1)),
        ScalarBuiltin::EndsWith => format!("({}).ends_with(({}).as_ref())", b(0), b(1)),
        ScalarBuiltin::Replace => format!(
            "match direct_string_replace({}, {}, {}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0),
            a(1),
            a(2)
        ),
        ScalarBuiltin::SplitBounded => format!(
            "match direct_split_bounded({}, {}, {}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0),
            a(1),
            a(2)
        ),
        ScalarBuiltin::Substring => {
            if args.len() == 2 {
                format!(
                    "match direct_substring({}, {}, None, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
                    a(0),
                    a(1)
                )
            } else {
                format!(
                    "match direct_substring({}, {}, Some({}), &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
                    a(0),
                    a(1),
                    a(2)
                )
            }
        }
        ScalarBuiltin::IndexOf => format!("direct_index_of({}, {}, false)", b(0), b(1)),
        ScalarBuiltin::LastIndexOf => format!("direct_index_of({}, {}, true)", b(0), b(1)),
        ScalarBuiltin::CharAt => format!(
            "match direct_char_at({}, {}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0),
            a(1)
        ),
        ScalarBuiltin::Repeat => format!(
            "match direct_repeat_string({}, {}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0),
            a(1)
        ),
        ScalarBuiltin::SafeHtmlEmpty => "std::sync::Arc::<str>::from(\"\")".into(),
        ScalarBuiltin::SafeHtmlText => format!(
            "match direct_safe_html_text({}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0)
        ),
        ScalarBuiltin::SafeHtmlElement => format!(
            "match direct_safe_html_element({}, {}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0),
            a(1)
        ),
        ScalarBuiltin::SafeHtmlLink => format!(
            "match direct_safe_html_link({}, {}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0),
            a(1)
        ),
        ScalarBuiltin::SafeHtmlConcat => format!(
            "match direct_concat_strings({}, {}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0),
            a(1)
        ),
        ScalarBuiltin::DictNew => "std::sync::Arc::new(std::collections::BTreeMap::new())".into(),
        ScalarBuiltin::ContainsKey => format!("({}).contains_key(({}).as_ref())", b(0), b(1)),
        ScalarBuiltin::RemoveKey => format!(
            "match direct_dict_remove_key({}, {}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}",
            a(0),
            a(1)
        ),
        ScalarBuiltin::Sin
        | ScalarBuiltin::Cos
        | ScalarBuiltin::Sqrt
        | ScalarBuiltin::MonotonicNanos
        | ScalarBuiltin::ToF32 => {
            unreachable!("typed scalar eligibility excludes numeric/timing builtins")
        }
    }
}

fn binary_source(
    body: &VerifiedScalarBody,
    left: &ScalarExpr,
    op: ScalarBinaryOp,
    right: &ScalarExpr,
) -> String {
    let l = expr_source(body, left);
    if op == ScalarBinaryOp::LogicalAnd {
        return format!("({l}) && ({})", expr_source(body, right));
    }
    if op == ScalarBinaryOp::LogicalOr {
        return format!("({l}) || ({})", expr_source(body, right));
    }
    let r = expr_source(body, right);
    match op {
        ScalarBinaryOp::Add => checked_int(&l, &r, "checked_add"),
        ScalarBinaryOp::Sub => checked_int(&l, &r, "checked_sub"),
        ScalarBinaryOp::Mul => checked_int(&l, &r, "checked_mul"),
        ScalarBinaryOp::Div => checked_int(&l, &r, "checked_div"),
        ScalarBinaryOp::Rem => checked_int(&l, &r, "checked_rem"),
        ScalarBinaryOp::BitAnd => format!("({l}) & ({r})"),
        ScalarBinaryOp::BitXor => format!("({l}) ^ ({r})"),
        ScalarBinaryOp::BitOr => format!("({l}) | ({r})"),
        ScalarBinaryOp::Lt => format!("({l}) < ({r})"),
        ScalarBinaryOp::Le => format!("({l}) <= ({r})"),
        ScalarBinaryOp::Gt => format!("({l}) > ({r})"),
        ScalarBinaryOp::Ge => format!("({l}) >= ({r})"),
        ScalarBinaryOp::Eq => format!("({l}) == ({r})"),
        ScalarBinaryOp::Ne => format!("({l}) != ({r})"),
        ScalarBinaryOp::StringConcat => format!(
            "match direct_concat_strings({l}, {r}, &mut velran_state) {{ Some(v) => v, None => return fail(&velran_fuel, &velran_state) }}"
        ),
        ScalarBinaryOp::LogicalAnd | ScalarBinaryOp::LogicalOr => unreachable!(),
    }
}

fn checked_int(left: &str, right: &str, method: &str) -> String {
    format!(
        "match ({left}).{method}({right}) {{ Some(v) => v, None => {{ velran_state.bad_request(); return fail(&velran_fuel, &velran_state); }} }}"
    )
}

fn expr_type(body: &VerifiedScalarBody, expr: &ScalarExpr) -> NativeScalarType {
    match expr {
        ScalarExpr::String(_) => NativeScalarType::String,
        ScalarExpr::Int(_) => NativeScalarType::Int,
        ScalarExpr::Bool(_) | ScalarExpr::Not(_) => NativeScalarType::Bool,
        ScalarExpr::Variable(name) => body.local_type(name).expect("verified variable type"),
        ScalarExpr::CollectionLen { .. } => NativeScalarType::Int,
        ScalarExpr::CollectionIndex { collection, .. } => match body
            .local_type(collection)
            .expect("verified collection type")
        {
            NativeScalarType::StringList | NativeScalarType::StringDict => NativeScalarType::String,
            _ => unreachable!(),
        },
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
            _ => expr_type(body, left),
        },
        ScalarExpr::Builtin { function, .. } => match function {
            ScalarBuiltin::StringLen | ScalarBuiltin::IndexOf | ScalarBuiltin::LastIndexOf => {
                NativeScalarType::Int
            }
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
            _ => unreachable!(),
        },
        ScalarExpr::SumPredicate { .. } => NativeScalarType::Bool,
        ScalarExpr::SumUnwrapOr { base_type, .. } => match base_type {
            NativeScalarType::Option(inner) => match inner {
                NativePureValueType::Int => NativeScalarType::Int,
                NativePureValueType::F32 => NativeScalarType::F32,
                NativePureValueType::Bool => NativeScalarType::Bool,
                NativePureValueType::String => NativeScalarType::String,
                NativePureValueType::StringList => NativeScalarType::StringList,
                NativePureValueType::SafeHtml => NativeScalarType::SafeHtml,
                NativePureValueType::Struct(id) => NativeScalarType::Struct(*id),
            },
            NativeScalarType::Result { ok, .. } => match ok {
                NativePureValueType::Int => NativeScalarType::Int,
                NativePureValueType::F32 => NativeScalarType::F32,
                NativePureValueType::Bool => NativeScalarType::Bool,
                NativePureValueType::String => NativeScalarType::String,
                NativePureValueType::StringList => NativeScalarType::StringList,
                NativePureValueType::SafeHtml => NativeScalarType::SafeHtml,
                NativePureValueType::Struct(id) => NativeScalarType::Struct(*id),
            },
            _ => unreachable!("verified unwrap_or type"),
        },
        _ => unreachable!(),
    }
}
