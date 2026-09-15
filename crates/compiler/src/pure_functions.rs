use crate::diagnostics::CompileError;
use crate::handler_types::StaticType;
use crate::module_namespace::qualify;
use crate::source_syntax::{function_bounds, is_identifier, split_top_level};
use language_core::{
    InlineHint, Program, PureFunction, PureFunctionParam, PureParamType, PureReturnType,
    PureValueType, ValueType,
};
use std::collections::HashMap;

const MAX_PURE_FIXED_ARRAY_LEN: u32 = 16_384;

pub(super) fn predeclare_pure_functions(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    let mut offset = 0usize;
    let mut pending_inline: Option<InlineHint> = None;

    for raw in source.split_inclusive('\n') {
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let trimmed = line.trim();
        let line_start = offset;
        offset += raw.len();

        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if let Some(hint) = parse_inline_attr(trimmed)? {
            let attr_rel = line.len() - line.trim_start().len();
            let attr_pos = line_start + attr_rel;
            if crate::declarations::is_top_level_declaration_at(source, attr_pos) {
                if pending_inline.replace(hint).is_some() {
                    return Err(CompileError::Syntax(
                        "multiple #[inline(...)] attributes before one function are not supported"
                            .into(),
                    ));
                }
            }
            continue;
        }

        let (fn_rel, inline) = if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            (
                line.find("fn ").expect("trimmed prefix exists"),
                pending_inline.take().unwrap_or(InlineHint::Unspecified),
            )
        } else if let Some(rest) = trimmed.strip_prefix("#[inline(never)] fn ") {
            let rel = line.find("fn ").expect("same-line fn exists");
            let _ = rest;
            (rel, InlineHint::Never)
        } else if let Some(rest) = trimmed.strip_prefix("#[inline(always)] fn ") {
            let rel = line.find("fn ").expect("same-line fn exists");
            let _ = rest;
            (rel, InlineHint::Always)
        } else if let Some(rest) = trimmed.strip_prefix("#[inline] fn ") {
            let rel = line.find("fn ").expect("same-line fn exists");
            let _ = rest;
            (rel, InlineHint::Hint)
        } else {
            if pending_inline.take().is_some() {
                return Err(CompileError::Syntax(
                    "#[inline(...)] must be followed by a normal pure `fn` declaration".into(),
                ));
            }
            continue;
        };

        let fn_pos = line_start + fn_rel;
        if !crate::declarations::is_top_level_declaration_at(source, fn_pos) {
            continue;
        }
        let start = fn_pos + "fn ".len();
        let (name, sig_open, sig_close, body_open, _body_close) =
            function_bounds(source, start, "pure function")?;
        let symbol_name = qualify(namespace, &name);
        ensure_unique(program, &symbol_name)?;
        let visibility = crate::visibility::declaration_visibility(source, fn_pos);
        crate::visibility::register(program, symbol_name.clone(), visibility);

        let suffix = source[sig_close + 1..body_open].trim();
        let return_type = parse_return_type(&name, suffix, namespace, program)?;
        let params = parse_params(&name, &source[sig_open + 1..sig_close], namespace, program)?;
        if visibility == language_core::Visibility::Public {
            crate::visibility::validate_public_pure_api(&name, &params, &return_type, program)?;
        }
        validate_param_family(&name, &params)?;
        if matches!(return_type, PureReturnType::Value(PureValueType::Struct(_)))
            && params
                .iter()
                .any(|param| matches!(param.ty, PureParamType::F32ArrayMut(_)))
        {
            return Err(CompileError::Syntax(format!(
                "pure function `{name}` struct return cannot currently cross the mutable numeric helper family; use scalar/string inputs or split the compute and struct assembly"
            )));
        }
        if matches!(
            return_type,
            PureReturnType::Option(_) | PureReturnType::Result { .. }
        ) && params
            .iter()
            .any(|param| matches!(param.ty, PureParamType::F32ArrayMut(_)))
        {
            return Err(CompileError::Syntax(format!(
                "pure function `{name}` Option/Result return cannot currently cross the mutable numeric helper family; split numeric compute from sum-value assembly"
            )));
        }
        program.pure_functions.push(PureFunction {
            name: symbol_name,
            params,
            return_type,
            inline,
            body: Vec::new(),
        });
    }

    if pending_inline.is_some() {
        return Err(CompileError::Syntax(
            "#[inline(...)] must be followed by a normal pure `fn` declaration".into(),
        ));
    }
    Ok(())
}

pub(super) fn lower_pure_function_bodies(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    let mut offset = 0usize;

    for raw in source.split_inclusive('\n') {
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let trimmed = line.trim();
        let line_start = offset;
        offset += raw.len();

        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#[inline") {
            continue;
        }

        let fn_rel = if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            line.find("fn ").expect("trimmed prefix exists")
        } else if trimmed.starts_with("#[inline(never)] fn ")
            || trimmed.starts_with("#[inline(always)] fn ")
            || trimmed.starts_with("#[inline] fn ")
        {
            line.find("fn ").expect("same-line fn exists")
        } else {
            continue;
        };

        let fn_pos = line_start + fn_rel;
        if !crate::declarations::is_top_level_declaration_at(source, fn_pos) {
            continue;
        }
        let start = fn_pos + "fn ".len();
        let (name, _sig_open, _sig_close, body_open, body_close) =
            function_bounds(source, start, "pure function")?;
        let symbol_name = qualify(namespace, &name);
        let function = program.pure_function(&symbol_name).ok_or_else(|| {
            CompileError::Syntax(format!("pure function `{symbol_name}` was not predeclared"))
        })?;
        let params = function.params.clone();
        let return_type = function.return_type.clone();

        let mut known = HashMap::new();
        for param in &params {
            let static_ty = match param.ty {
                PureParamType::F32ArrayMut(_) => StaticType::trusted_scalar(ValueType::F32Array),
                PureParamType::Int => StaticType::trusted_scalar(ValueType::Int),
                PureParamType::Bool => StaticType::trusted_scalar(ValueType::Bool),
                PureParamType::Str => StaticType::trusted_scalar(ValueType::String),
                PureParamType::StringList => StaticType::trusted_scalar(ValueType::StringList),
                PureParamType::Struct(id) => {
                    let schema = program.json_schema_by_id(id).ok_or_else(|| {
                        CompileError::Syntax(format!(
                            "pure function `{name}` references unknown struct parameter id {id}"
                        ))
                    })?;
                    StaticType::PureStruct(schema.name.clone())
                }
            };
            known.insert(param.name.clone(), static_ty);
        }
        let body_line = source[..body_open + 1]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            + 1;
        let body = crate::control_flow::parse_pure_compute_statements(
            &symbol_name,
            namespace,
            &source[body_open + 1..body_close],
            &mut known,
            program,
            body_line,
        )?;
        validate_param_bindings(&name, &params, &body)?;
        validate_return_contract(&name, &return_type, &body, &known, program)?;
        let function = program
            .pure_functions
            .iter_mut()
            .find(|function| function.name == symbol_name)
            .expect("predeclared pure function remains registered");
        function.body = body;
    }
    Ok(())
}

pub(super) fn parse_pure_functions(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    predeclare_pure_functions(source, namespace, program)?;
    lower_pure_function_bodies(source, namespace, program)
}

pub(super) fn validate_pure_recursion_contract(program: &Program) -> Result<(), CompileError> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mark {
        Visiting,
        Done,
    }

    fn collect_calls(statements: &[language_core::ComputeStatement], out: &mut Vec<String>) {
        for statement in statements {
            match statement {
                language_core::ComputeStatement::PureCall { function, .. } => {
                    out.push(function.clone())
                }
                language_core::ComputeStatement::While { statements, .. }
                | language_core::ComputeStatement::If { statements, .. } => {
                    collect_calls(statements, out)
                }
                _ => {}
            }
        }
    }

    fn visit(
        name: &str,
        program: &Program,
        marks: &mut HashMap<String, Mark>,
        stack: &mut Vec<String>,
    ) -> Result<(), CompileError> {
        match marks.get(name).copied() {
            Some(Mark::Done) => return Ok(()),
            Some(Mark::Visiting) => {
                let start = stack.iter().position(|item| item == name).unwrap_or(0);
                let cycle = &stack[start..];
                let touches_unbounded_numeric_path = cycle.iter().any(|item| {
                    program.pure_function(item).is_some_and(|function| {
                        function
                            .params
                            .iter()
                            .any(|param| matches!(param.ty, PureParamType::F32ArrayMut(_)))
                    })
                });
                if touches_unbounded_numeric_path {
                    let mut path = cycle.to_vec();
                    path.push(name.to_string());
                    return Err(CompileError::Syntax(format!(
                        "recursive pure call cycle through the numeric hot path is not allowed: {}",
                        path.join(" -> ")
                    )));
                }
                // Owned/borrowed scalar pure recursion is runtime depth-bounded.
                return Ok(());
            }
            None => {}
        }
        marks.insert(name.to_string(), Mark::Visiting);
        stack.push(name.to_string());
        if let Some(function) = program.pure_function(name) {
            let mut calls = Vec::new();
            collect_calls(&function.body, &mut calls);
            for callee in calls {
                visit(&callee, program, marks, stack)?;
            }
        }
        stack.pop();
        marks.insert(name.to_string(), Mark::Done);
        Ok(())
    }

    // Keep the mutable fixed-array hot path ABI closed over itself. Its generated
    // helpers use the primitive tuple transport, while scalar helpers use the owned
    // Scalar transport and recursion budget. Crossing that ABI boundary from inside
    // another pure helper is rejected until a unified internal call ABI exists.
    for function in &program.pure_functions {
        let numeric_family = function
            .params
            .iter()
            .any(|param| matches!(param.ty, PureParamType::F32ArrayMut(_)));
        let mut calls = Vec::new();
        collect_calls(&function.body, &mut calls);
        for callee in calls {
            let Some(target) = program.pure_function(&callee) else {
                continue;
            };
            let target_numeric_family = target
                .params
                .iter()
                .any(|param| matches!(param.ty, PureParamType::F32ArrayMut(_)));
            if numeric_family != target_numeric_family {
                return Err(CompileError::Syntax(format!(
                    "pure helper `{}` cannot cross the scalar/numeric helper ABI when calling `{callee}`; keep scalar orchestration and mutable numeric kernels separate",
                    function.name
                )));
            }
        }
    }

    let mut marks = HashMap::new();
    for function in &program.pure_functions {
        let mut stack = Vec::new();
        visit(&function.name, program, &mut marks, &mut stack)?;
    }
    Ok(())
}

fn parse_return_type(
    name: &str,
    suffix: &str,
    namespace: &str,
    program: &Program,
) -> Result<PureReturnType, CompileError> {
    let compact: String = suffix.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() || compact == "->()" {
        return Ok(PureReturnType::Unit);
    }
    let Some(raw) = compact.strip_prefix("->") else {
        return Err(CompileError::Syntax(format!(
            "pure function `{name}` has invalid return type syntax"
        )));
    };
    if let Some(inner) = crate::generic_type_syntax::unwrap_generic(raw, "Option") {
        return Ok(PureReturnType::Option(parse_pure_value_type(
            name, inner, namespace, program,
        )?));
    }
    if let Some(inner) = crate::generic_type_syntax::unwrap_generic(raw, "Result") {
        let parts = split_top_level(inner, ',');
        if parts.len() != 2 {
            return Err(CompileError::Syntax(format!(
                "pure function `{name}` Result return requires exactly two type arguments"
            )));
        }
        return Ok(PureReturnType::Result {
            ok: parse_pure_value_type(name, &parts[0], namespace, program)?,
            err: parse_pure_value_type(name, &parts[1], namespace, program)?,
        });
    }
    Ok(PureReturnType::Value(parse_pure_value_type(
        name, raw, namespace, program,
    )?))
}

fn parse_pure_value_type(
    function_name: &str,
    raw: &str,
    namespace: &str,
    program: &Program,
) -> Result<PureValueType, CompileError> {
    let raw = raw.trim();
    match raw {
        "i64" => Ok(PureValueType::Int),
        "f32" => Ok(PureValueType::F32),
        "bool" => Ok(PureValueType::Bool),
        "String" => Ok(PureValueType::String),
        "Vec<String>" => Ok(PureValueType::StringList),
        "SafeHtml" => Ok(PureValueType::SafeHtml),
        _ if crate::module_namespace::is_symbol_path(raw) => {
            let symbol = qualify(namespace, raw);
            if program.json_schema(&symbol).is_some() {
                crate::visibility::require_access(program, &symbol, namespace, "struct")?;
                Ok(PureValueType::Struct(symbol))
            } else {
                Err(CompileError::Syntax(format!(
                    "pure function `{function_name}` return type `{raw}` is not a known safe value type or struct"
                )))
            }
        }
        _ => Err(CompileError::Syntax(format!(
            "pure function `{function_name}` return type `{raw}` is unsupported; safe value types are currently i64, f32, bool, String, Vec<String>, SafeHtml, and declared structs"
        ))),
    }
}

fn validate_return_contract(
    name: &str,
    expected: &PureReturnType,
    body: &[language_core::ComputeStatement],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    fn validate_nested_returns(
        name: &str,
        expected: &PureReturnType,
        statements: &[language_core::ComputeStatement],
        known: &HashMap<String, StaticType>,
        program: &Program,
    ) -> Result<(), CompileError> {
        for statement in statements {
            match statement {
                language_core::ComputeStatement::If { statements, .. }
                | language_core::ComputeStatement::While { statements, .. } => {
                    validate_nested_returns(name, expected, statements, known, program)?;
                }
                language_core::ComputeStatement::Return(expr) => {
                    let PureReturnType::Value(value) = expected else {
                        return Err(CompileError::Syntax(format!(
                            "pure function `{name}` nested return does not match its declared return type"
                        )));
                    };
                    let Some(expected_value) = pure_value_to_value_type(value) else {
                        return Err(CompileError::Syntax(format!(
                            "pure function `{name}` nested struct return must use `return StructName {{ ... }};`"
                        )));
                    };
                    if crate::expression::infer_expr_type(expr, known, program)? != expected_value {
                        return Err(CompileError::Syntax(format!(
                            "pure function `{name}` nested return type mismatch"
                        )));
                    }
                }
                language_core::ComputeStatement::ReturnStruct { schema, .. } => match expected {
                    PureReturnType::Value(PureValueType::Struct(expected_schema))
                        if schema == expected_schema => {}
                    _ => {
                        return Err(CompileError::Syntax(format!(
                            "pure function `{name}` nested struct return does not match its declared return type"
                        )));
                    }
                },
                language_core::ComputeStatement::ReturnOption { value } => {
                    let PureReturnType::Option(inner) = expected else {
                        return Err(CompileError::Syntax(format!(
                            "pure function `{name}` nested Option return does not match its declared return type"
                        )));
                    };
                    if let Some(expr) = value {
                        let expected_value = pure_value_to_value_type(inner).ok_or_else(|| {
                            CompileError::Syntax(format!(
                                "pure function `{name}` Option<struct> lowering is not enabled yet"
                            ))
                        })?;
                        if crate::expression::infer_expr_type(expr, known, program)?
                            != expected_value
                        {
                            return Err(CompileError::Syntax(format!(
                                "pure function `{name}` nested Option payload type mismatch"
                            )));
                        }
                    }
                }
                language_core::ComputeStatement::ReturnResult { is_ok, value } => {
                    let PureReturnType::Result { ok, err } = expected else {
                        return Err(CompileError::Syntax(format!(
                            "pure function `{name}` nested Result return does not match its declared return type"
                        )));
                    };
                    let value_ty = if *is_ok { ok } else { err };
                    let expected_value = pure_value_to_value_type(value_ty).ok_or_else(|| {
                        CompileError::Syntax(format!(
                            "pure function `{name}` Result<struct,...> lowering is not enabled yet"
                        ))
                    })?;
                    if crate::expression::infer_expr_type(value, known, program)? != expected_value
                    {
                        return Err(CompileError::Syntax(format!(
                            "pure function `{name}` nested Result payload type mismatch"
                        )));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    for statement in body {
        if let language_core::ComputeStatement::If { statements, .. }
        | language_core::ComputeStatement::While { statements, .. } = statement
        {
            validate_nested_returns(name, expected, statements, known, program)?;
        }
    }
    match expected {
        PureReturnType::Unit => {
            if body.iter().any(|s| {
                matches!(
                    s,
                    language_core::ComputeStatement::Return(_)
                        | language_core::ComputeStatement::ReturnStruct { .. }
                        | language_core::ComputeStatement::ReturnOption { .. }
                        | language_core::ComputeStatement::ReturnResult { .. }
                )
            }) {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` returning `()` cannot return a value"
                )));
            }
        }
        PureReturnType::Value(PureValueType::Struct(expected_schema)) => {
            let Some(language_core::ComputeStatement::ReturnStruct { schema, .. }) = body.last()
            else {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` returning a struct must end with `return StructName {{ ... }};`"
                )));
            };
            if schema != expected_schema {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` returns struct `{schema}` but declares `{expected_schema}`"
                )));
            }
            if body[..body.len() - 1].iter().any(|s| {
                matches!(
                    s,
                    language_core::ComputeStatement::Return(_)
                        | language_core::ComputeStatement::ReturnStruct { .. }
                        | language_core::ComputeStatement::ReturnOption { .. }
                        | language_core::ComputeStatement::ReturnResult { .. }
                )
            }) {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` may only return once in this iteration"
                )));
            }
        }
        PureReturnType::Value(expected) => {
            let Some(language_core::ComputeStatement::Return(expr)) = body.last() else {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` must end with `return <expr>;`"
                )));
            };
            if body[..body.len() - 1].iter().any(|s| {
                matches!(
                    s,
                    language_core::ComputeStatement::Return(_)
                        | language_core::ComputeStatement::ReturnStruct { .. }
                        | language_core::ComputeStatement::ReturnOption { .. }
                        | language_core::ComputeStatement::ReturnResult { .. }
                )
            }) {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` may only return once in this iteration"
                )));
            }
            let actual = crate::expression::infer_expr_type(expr, known, program)?;
            let expected_value = match expected {
                PureValueType::Int => ValueType::Int,
                PureValueType::F32 => ValueType::F32,
                PureValueType::Bool => ValueType::Bool,
                PureValueType::String => ValueType::String,
                PureValueType::StringList => ValueType::StringList,
                PureValueType::SafeHtml => ValueType::Domain(language_core::SAFE_HTML_DOMAIN_ID),
                PureValueType::Struct(_) => unreachable!(),
            };
            if actual != expected_value {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` return type mismatch"
                )));
            }
        }
        PureReturnType::Option(inner) => {
            let Some(last) = body.last() else {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` returning `Option<T>` must end with `return Some(...);` or `return None;`"
                )));
            };
            match last {
                language_core::ComputeStatement::ReturnOption { value: None } => {}
                language_core::ComputeStatement::ReturnOption { value: Some(expr) } => {
                    let actual = crate::expression::infer_expr_type(expr, known, program)?;
                    let expected_value = pure_value_to_value_type(inner).ok_or_else(|| {
                        CompileError::Syntax(format!(
                            "pure function `{name}` Option<struct> lowering is not enabled yet"
                        ))
                    })?;
                    if actual != expected_value {
                        return Err(CompileError::Syntax(format!(
                            "pure function `{name}` Option payload type mismatch"
                        )));
                    }
                }
                _ => {
                    return Err(CompileError::Syntax(format!(
                        "pure function `{name}` returning `Option<T>` must end with `return Some(...);` or `return None;`"
                    )));
                }
            }
            if body[..body.len() - 1]
                .iter()
                .any(|s| is_return_statement(s))
            {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` may only return once in this iteration"
                )));
            }
        }
        PureReturnType::Result { ok, err } => {
            let Some(language_core::ComputeStatement::ReturnResult { is_ok, value }) = body.last()
            else {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` returning `Result<T,E>` must end with `return Ok(...);` or `return Err(...);`"
                )));
            };
            let expected = if *is_ok { ok } else { err };
            let expected_value = pure_value_to_value_type(expected).ok_or_else(|| {
                CompileError::Syntax(format!(
                    "pure function `{name}` Result<struct,...> lowering is not enabled yet"
                ))
            })?;
            let actual = crate::expression::infer_expr_type(value, known, program)?;
            if actual != expected_value {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` Result payload type mismatch"
                )));
            }
            if body[..body.len() - 1]
                .iter()
                .any(|s| is_return_statement(s))
            {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` may only return once in this iteration"
                )));
            }
        }
    }
    Ok(())
}

fn is_return_statement(statement: &language_core::ComputeStatement) -> bool {
    matches!(
        statement,
        language_core::ComputeStatement::Return(_)
            | language_core::ComputeStatement::ReturnStruct { .. }
            | language_core::ComputeStatement::ReturnOption { .. }
            | language_core::ComputeStatement::ReturnResult { .. }
    )
}

fn pure_value_to_value_type(value: &PureValueType) -> Option<ValueType> {
    Some(match value {
        PureValueType::Int => ValueType::Int,
        PureValueType::F32 => ValueType::F32,
        PureValueType::Bool => ValueType::Bool,
        PureValueType::String => ValueType::String,
        PureValueType::StringList => ValueType::StringList,
        PureValueType::SafeHtml => ValueType::Domain(language_core::SAFE_HTML_DOMAIN_ID),
        PureValueType::Struct(_) => return None,
    })
}

fn validate_param_bindings(
    name: &str,
    params: &[PureFunctionParam],
    body: &[language_core::ComputeStatement],
) -> Result<(), CompileError> {
    fn walk(
        params: &[PureFunctionParam],
        body: &[language_core::ComputeStatement],
    ) -> Option<String> {
        for statement in body {
            match statement {
                language_core::ComputeStatement::Let { name, .. }
                | language_core::ComputeStatement::Set { name, .. }
                    if params.iter().any(|p| p.name == *name) =>
                {
                    return Some(name.clone());
                }
                language_core::ComputeStatement::While { statements, .. }
                | language_core::ComputeStatement::If { statements, .. } => {
                    if let Some(hit) = walk(params, statements) {
                        return Some(hit);
                    }
                }
                _ => {}
            }
        }
        None
    }
    if let Some(param) = walk(params, body) {
        return Err(CompileError::Syntax(format!(
            "pure function `{name}` may mutate parameter `{param}` elements but may not rebind or shadow the parameter"
        )));
    }
    Ok(())
}

fn parse_inline_attr(trimmed: &str) -> Result<Option<InlineHint>, CompileError> {
    if trimmed == "#[inline]" {
        return Ok(Some(InlineHint::Hint));
    }
    if trimmed == "#[inline(always)]" {
        return Ok(Some(InlineHint::Always));
    }
    if trimmed == "#[inline(never)]" {
        return Ok(Some(InlineHint::Never));
    }
    if trimmed.starts_with("#[inline") {
        return Err(CompileError::Syntax(
            "unsupported inline attribute; allowed: #[inline], #[inline(always)], #[inline(never)]"
                .into(),
        ));
    }
    Ok(None)
}

fn parse_params(
    name: &str,
    raw: &str,
    namespace: &str,
    program: &Program,
) -> Result<Vec<PureFunctionParam>, CompileError> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for part in split_top_level(raw, ',') {
        let (param_name, ty) = part.split_once(':').ok_or_else(|| {
            CompileError::Syntax(format!(
                "pure function `{name}` parameter requires `name: type`"
            ))
        })?;
        let param_name = param_name.trim();
        if !is_identifier(param_name)
            || out.iter().any(|p: &PureFunctionParam| p.name == param_name)
        {
            return Err(CompileError::Syntax(format!(
                "pure function `{name}` has invalid or duplicate parameter `{param_name}`"
            )));
        }
        let compact: String = ty.chars().filter(|ch| !ch.is_whitespace()).collect();
        let param_ty = if compact == "i64" {
            PureParamType::Int
        } else if compact == "bool" {
            PureParamType::Bool
        } else if compact == "&str" {
            PureParamType::Str
        } else if compact == "&[String]" {
            PureParamType::StringList
        } else if let Some(inner) = compact
            .strip_prefix("&mut[f32;")
            .and_then(|v| v.strip_suffix(']'))
        {
            let array_len: u32 = inner.parse().map_err(|_| {
                CompileError::Syntax(format!(
                    "pure function `{name}` parameter `{param_name}` has invalid fixed array length"
                ))
            })?;
            if !(1..=MAX_PURE_FIXED_ARRAY_LEN).contains(&array_len) {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` parameter `{param_name}` array length must be 1..={MAX_PURE_FIXED_ARRAY_LEN}"
                )));
            }
            PureParamType::F32ArrayMut(array_len)
        } else if let Some(struct_name) = compact.strip_prefix('&') {
            if !crate::module_namespace::is_symbol_path(struct_name) {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` parameter `{param_name}` has invalid borrowed struct path `{struct_name}`"
                )));
            }
            let symbol = crate::module_namespace::resolve(namespace, struct_name);
            let Some((id, _)) = program.json_schema_by_name(&symbol) else {
                return Err(CompileError::Syntax(format!(
                    "pure function `{name}` parameter `{param_name}` references unknown borrowed struct `{struct_name}`"
                )));
            };
            crate::visibility::require_access(program, &symbol, namespace, "struct")?;
            PureParamType::Struct(id)
        } else {
            return Err(CompileError::Syntax(format!(
                "pure function `{name}` parameter `{param_name}` must use `i64`, `bool`, `&str`, `&[String]`, `&Struct`, or `&mut [f32; N]`"
            )));
        };
        out.push(PureFunctionParam {
            name: param_name.into(),
            ty: param_ty,
        });
    }
    Ok(out)
}

fn validate_param_family(name: &str, params: &[PureFunctionParam]) -> Result<(), CompileError> {
    let has_arrays = params
        .iter()
        .any(|p| matches!(p.ty, PureParamType::F32ArrayMut(_)));
    let has_scalar_or_borrowed = params.iter().any(|p| {
        matches!(
            p.ty,
            PureParamType::Int
                | PureParamType::Bool
                | PureParamType::Str
                | PureParamType::StringList
                | PureParamType::Struct(_)
        )
    });
    if has_arrays && has_scalar_or_borrowed {
        return Err(CompileError::Syntax(format!(
            "pure function `{name}` cannot mix scalar/borrowed parameters and `&mut [f32; N]` parameters in this iteration"
        )));
    }
    Ok(())
}

fn ensure_unique(program: &Program, name: &str) -> Result<(), CompileError> {
    if program.pure_function(name).is_some()
        || program.page(name).is_some()
        || program.action(name).is_some()
        || program.component(name).is_some()
        || program.layout(name).is_some()
        || program.query(name).is_some()
    {
        return Err(CompileError::DuplicateHandler(name.into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_hint_on_separate_line() {
        let mut p = Program::default();
        parse_pure_functions(
            "#[inline]\nfn helper(real: &mut [f32; 16]) { real[0] = 1.0f32; }\n",
            "",
            &mut p,
        )
        .unwrap();
        assert_eq!(p.pure_functions[0].inline, InlineHint::Hint);
    }

    #[test]
    fn parses_inline_never_fixed_array_function() {
        let mut p = Program::default();
        parse_pure_functions(
            "#[inline(never)]\nfn fft(real: &mut [f32; 4096]) { real[0] = 1.0f32; }\n",
            "",
            &mut p,
        )
        .unwrap();
        assert_eq!(p.pure_functions.len(), 1);
        assert_eq!(p.pure_functions[0].inline, InlineHint::Never);
        assert_eq!(
            p.pure_functions[0].params[0].ty,
            PureParamType::F32ArrayMut(4096)
        );
    }

    #[test]
    fn rejects_ambient_or_unverified_parameter_types() {
        let mut p = Program::default();
        let err = parse_pure_functions("fn bad(path: String) { }\n", "", &mut p).unwrap_err();
        assert!(
            err.to_string()
                .contains("`i64`, `bool`, `&str`, `&[String]`, `&Struct`, or `&mut [f32; N]")
        );
    }

    #[test]
    fn parses_borrowed_string_parameter() {
        let mut p = Program::default();
        parse_pure_functions(
            "fn trimmed_len(input: &str) -> i64 { return input.trim().chars().count(); }\n",
            "",
            &mut p,
        )
        .unwrap();
        assert_eq!(p.pure_functions[0].params[0].ty, PureParamType::Str);
        assert_eq!(
            p.pure_functions[0].return_type,
            PureReturnType::Value(PureValueType::Int)
        );
    }

    #[test]
    fn parses_owned_string_return() {
        let mut p = Program::default();
        parse_pure_functions(
            "fn normalize(input: &str) -> String { return input.trim().to_lowercase(); }\n",
            "",
            &mut p,
        )
        .unwrap();
        assert_eq!(p.pure_functions[0].params[0].ty, PureParamType::Str);
        assert_eq!(
            p.pure_functions[0].return_type,
            PureReturnType::Value(PureValueType::String)
        );
    }

    #[test]
    fn parses_scalar_value_parameters() {
        let mut p = Program::default();
        parse_pure_functions(
            "fn choose(value: i64, enabled: bool) -> i64 { if enabled { return value; } return 0; }\n",
            "",
            &mut p,
        )
        .unwrap();
        assert_eq!(p.pure_functions[0].params[0].ty, PureParamType::Int);
        assert_eq!(p.pure_functions[0].params[1].ty, PureParamType::Bool);
    }

    #[test]
    fn scalar_pure_functions_can_call_helpers_with_expression_arguments() {
        let mut p = Program::default();
        parse_pure_functions(
            "fn step(value: i64, enabled: bool) -> i64 { if enabled { return value + 1; } return value; }\nfn run(value: i64) -> i64 { let first = step(value + 1, true); let mut out = 0; out = step(first, false); return out; }\n",
            "",
            &mut p,
        )
        .unwrap();
        validate_pure_recursion_contract(&p).unwrap();
        assert!(p.pure_function("run").is_some());
    }

    #[test]
    fn scalar_recursion_is_allowed_but_numeric_hot_path_recursion_is_rejected() {
        let mut scalar = Program::default();
        parse_pure_functions(
            "fn nested(input: &str) -> String { if input.len() == 0 { return input.to_string(); } return nested(input); }\n",
            "",
            &mut scalar,
        )
        .unwrap();
        validate_pure_recursion_contract(&scalar).unwrap();

        let mut numeric = Program::default();
        parse_pure_functions(
            "fn spin(real: &mut [f32; 16]) -> i64 { return spin(&mut real); }\n",
            "",
            &mut numeric,
        )
        .unwrap();
        let err = validate_pure_recursion_contract(&numeric).unwrap_err();
        assert!(err.to_string().contains("numeric hot path"));
    }

    #[test]
    fn rejects_mixed_string_and_mutable_array_parameters() {
        let mut p = Program::default();
        let err = parse_pure_functions(
            "fn mixed(input: &str, real: &mut [f32; 16]) -> i64 { return input.len(); }\n",
            "",
            &mut p,
        )
        .unwrap_err();
        assert!(err.to_string().contains("cannot mix"));
    }

    #[test]
    fn parses_typed_scalar_return() {
        let mut p = Program::default();
        parse_pure_functions(
            "fn bin(real: &mut [f32; 16]) -> f32 { return real[1]; }\n",
            "",
            &mut p,
        )
        .unwrap();
        assert_eq!(
            p.pure_functions[0].return_type,
            PureReturnType::Value(PureValueType::F32)
        );
    }

    #[test]
    fn rejects_non_terminal_typed_return() {
        let mut p = Program::default();
        let err = parse_pure_functions(
            "fn bad(real: &mut [f32; 16]) -> f32 { return real[1]; real[0] = 1.0f32; }\n",
            "",
            &mut p,
        )
        .unwrap_err();
        assert!(err.to_string().contains("must end"));
    }

    #[test]
    fn parses_string_list_borrow_and_owned_return() {
        let mut p = Program::default();
        parse_pure_functions(
            "fn echo_tags(tags: &[String]) -> Vec<String> { return tags; }\n",
            "",
            &mut p,
        )
        .unwrap();
        assert_eq!(p.pure_functions[0].params[0].ty, PureParamType::StringList);
        assert_eq!(
            p.pure_functions[0].return_type,
            PureReturnType::Value(PureValueType::StringList)
        );
    }

    #[test]
    fn recognizes_option_result_and_struct_return_contracts_before_lowering() {
        let mut p = Program::default();
        p.json_schemas.push(language_core::JsonSchema {
            name: "Pair".into(),
            fields: vec![
                language_core::FormField {
                    name: "name".into(),
                    ty: ValueType::String,
                },
                language_core::FormField {
                    name: "count".into(),
                    ty: ValueType::Int,
                },
            ],
        });
        crate::visibility::register(&mut p, "Pair", language_core::Visibility::Public);
        assert_eq!(
            parse_return_type("maybe", "-> Option<String>", "", &p).unwrap(),
            PureReturnType::Option(PureValueType::String)
        );
        assert_eq!(
            parse_return_type("fallible", concat!("-> Result<i64, ", "String>"), "", &p).unwrap(),
            PureReturnType::Result {
                ok: PureValueType::Int,
                err: PureValueType::String
            }
        );
        assert_eq!(
            parse_return_type("pair", "-> Pair", "", &p).unwrap(),
            PureReturnType::Value(PureValueType::Struct("Pair".into()))
        );
    }
}
