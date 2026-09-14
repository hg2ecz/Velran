use crate::diagnostics::CompileError;
use crate::handler_types::StaticType;
use crate::module_namespace::resolve;
use crate::source_syntax::{find_statement_end, is_identifier, matching_paren, split_top_level};
use language_core::{Program, PureParamType, PureReturnType, PureValueType, ValueType};
use std::collections::HashMap;

pub(super) fn parse(
    body: &str,
    cursor: usize,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<Option<(String, Vec<String>, usize)>, CompileError> {
    let end = match find_statement_end(body, cursor) {
        Ok(end) => end,
        Err(_) => return Ok(None),
    };
    let text = body[cursor..end].trim();
    let Some(open_rel) = text.find('(') else {
        return Ok(None);
    };
    if !text.ends_with(')') {
        return Ok(None);
    }
    let name = text[..open_rel].trim();
    if !valid_call_name(name) {
        return Ok(None);
    }
    let open = cursor + body[cursor..end].find('(').expect("relative open exists");
    let Some(close) = matching_paren(body, open) else {
        return Ok(None);
    };
    // Be conservative: this parser only owns a whole direct call statement.
    if !body[close + 1..end].trim().is_empty() {
        return Ok(None);
    }
    let Some((symbol, mut implicit_args)) = resolve_pure_call(name, namespace, known, program)?
    else {
        return Ok(None);
    };
    let Some(function) = program.pure_function(&symbol) else {
        return Ok(None);
    };
    if function.return_type != PureReturnType::Unit {
        return Err(CompileError::Syntax(format!(
            "pure function `{name}` returns a value; bind it with `let value = {name}(...);`"
        )));
    }
    let raw_args = &body[open + 1..close];
    let parts = if raw_args.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level(raw_args, ',')
    };
    implicit_args.extend(parts.iter().map(|part| part.trim().to_string()));
    let refs = implicit_args.iter().map(String::as_str).collect::<Vec<_>>();
    let args = validate_args(name, &refs, function, known, program)?;
    Ok(Some((symbol, args, end + 1)))
}

pub(super) fn parse_value_call(
    text: &str,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<Option<(String, Vec<String>, StaticType)>, CompileError> {
    let text = text.trim();
    let Some(open_rel) = text.find('(') else {
        return Ok(None);
    };
    if !text.ends_with(')') {
        return Ok(None);
    }
    let name = text[..open_rel].trim();
    if !valid_call_name(name) {
        return Ok(None);
    }
    let Some((symbol, mut implicit_args)) = resolve_pure_call(name, namespace, known, program)?
    else {
        return Ok(None);
    };
    let Some(function) = program.pure_function(&symbol) else {
        return Ok(None);
    };
    if function.return_type == PureReturnType::Unit {
        return Ok(None);
    }
    let raw_args = &text[open_rel + 1..text.len() - 1];
    let parts = if raw_args.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level(raw_args, ',')
    };
    implicit_args.extend(parts.iter().map(|part| part.trim().to_string()));
    let refs = implicit_args.iter().map(String::as_str).collect::<Vec<_>>();
    let args = validate_args(name, &refs, function, known, program)?;
    let return_type = match &function.return_type {
        PureReturnType::Value(PureValueType::Int) => StaticType::trusted_scalar(ValueType::Int),
        PureReturnType::Value(PureValueType::F32) => StaticType::trusted_scalar(ValueType::F32),
        PureReturnType::Value(PureValueType::Bool) => StaticType::trusted_scalar(ValueType::Bool),
        PureReturnType::Value(PureValueType::String) => {
            StaticType::trusted_scalar(ValueType::String)
        }
        PureReturnType::Value(PureValueType::StringList) => {
            StaticType::trusted_scalar(ValueType::StringList)
        }
        PureReturnType::Value(PureValueType::Struct(schema)) => {
            StaticType::PureStruct(schema.clone())
        }
        PureReturnType::Option(inner) => StaticType::PureOption(inner.clone()),
        PureReturnType::Result { ok, err } => StaticType::PureResult {
            ok: ok.clone(),
            err: err.clone(),
        },
        PureReturnType::Unit => unreachable!(),
    };
    Ok(Some((symbol, args, return_type)))
}

fn valid_call_name(name: &str) -> bool {
    if let Some((receiver, method)) = name.split_once('.') {
        return !receiver.contains('.') && is_identifier(receiver) && is_identifier(method);
    }
    name.split("::").all(is_identifier)
}

fn resolve_pure_call(
    name: &str,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<Option<(String, Vec<String>)>, CompileError> {
    if let Some((receiver, method_name)) = name.split_once('.') {
        let Some(StaticType::PureStruct(target)) = known.get(receiver) else {
            return Ok(None);
        };
        let Some(method) = program.inherent_method(target, method_name) else {
            return Err(CompileError::Syntax(format!(
                "unknown inherent method `{method_name}` for struct `{target}`"
            )));
        };
        crate::visibility::require_access(
            program,
            &format!("{target}::{method_name}"),
            namespace,
            "method",
        )?;
        if !method.has_receiver {
            return Err(CompileError::Syntax(format!(
                "associated function `{target}::{method_name}` has no `&self` receiver; call it with the type name"
            )));
        }
        return Ok(Some((
            method.function.clone(),
            vec![format!("&{receiver}")],
        )));
    }

    if let Some((raw_target, method_name)) = name.rsplit_once("::") {
        let target = resolve(namespace, raw_target);
        if let Some(method) = program.inherent_method(&target, method_name) {
            crate::visibility::require_access(
                program,
                &format!("{target}::{method_name}"),
                namespace,
                "method",
            )?;
            if method.has_receiver {
                return Err(CompileError::Syntax(format!(
                    "method `{name}` requires a receiver; call it as `value.{method_name}(...)`"
                )));
            }
            return Ok(Some((method.function.clone(), Vec::new())));
        }
    }

    let symbol = resolve(namespace, name);
    if program.pure_function(&symbol).is_some() {
        crate::visibility::require_access(program, &symbol, namespace, "function")?;
        Ok(Some((symbol, Vec::new())))
    } else {
        Ok(None)
    }
}

fn validate_args(
    name: &str,
    parts: &[&str],
    function: &language_core::PureFunction,
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<Vec<String>, CompileError> {
    if parts.len() != function.params.len() {
        return Err(CompileError::Syntax(format!(
            "pure function `{name}` expects {} arguments, got {}",
            function.params.len(),
            parts.len()
        )));
    }
    let mut args = Vec::with_capacity(parts.len());
    let mut mutable_args = Vec::new();
    for (part, param) in parts.iter().zip(&function.params) {
        let part = part.trim();
        let variable = match param.ty {
            PureParamType::F32ArrayMut(_) => {
                let Some(variable) = part.strip_prefix("&mut ").map(str::trim) else {
                    return Err(CompileError::Syntax(format!(
                        "pure function `{name}` argument `{}` must be passed as `&mut variable`",
                        param.name
                    )));
                };
                if !is_identifier(variable)
                    || !known
                        .get(variable)
                        .is_some_and(|ty| ty.is_scalar(ValueType::F32Array))
                {
                    return Err(CompileError::Syntax(format!(
                        "pure function `{name}` argument `{variable}` must be a local f32 array"
                    )));
                }
                if mutable_args.iter().any(|existing| existing == variable) {
                    return Err(CompileError::Syntax(format!(
                        "pure function `{name}` cannot borrow `{variable}` mutably more than once in one call"
                    )));
                }
                mutable_args.push(variable.to_string());
                variable
            }
            PureParamType::Str => {
                let Some(variable) = part.strip_prefix('&').map(str::trim) else {
                    return Err(CompileError::Syntax(format!(
                        "pure function `{name}` argument `{}` has type `&str` and must be passed as `&variable`",
                        param.name
                    )));
                };
                if variable.starts_with("mut ")
                    || !is_identifier(variable)
                    || !known
                        .get(variable)
                        .is_some_and(|ty| ty.is_scalar(ValueType::String))
                {
                    return Err(CompileError::Syntax(format!(
                        "pure function `{name}` argument `{}` must borrow a local string as `&variable`",
                        param.name
                    )));
                }
                variable
            }
            PureParamType::StringList => {
                let Some(variable) = part.strip_prefix('&').map(str::trim) else {
                    return Err(CompileError::Syntax(format!(
                        "pure function `{name}` argument `{}` has type `&[String]` and must be passed as `&variable`",
                        param.name
                    )));
                };
                if variable.starts_with("mut ")
                    || !is_identifier(variable)
                    || !known
                        .get(variable)
                        .is_some_and(|ty| ty.is_scalar(ValueType::StringList))
                {
                    return Err(CompileError::Syntax(format!(
                        "pure function `{name}` argument `{}` must borrow a local `Vec<String>` as `&variable`",
                        param.name
                    )));
                }
                variable
            }
            PureParamType::Struct(id) => {
                let Some(variable) = part.strip_prefix('&').map(str::trim) else {
                    return Err(CompileError::Syntax(format!(
                        "pure function `{name}` argument `{}` is a borrowed struct and must be passed as `&variable`",
                        param.name
                    )));
                };
                let expected = program
                    .json_schema_by_id(id)
                    .map(|schema| schema.name.as_str())
                    .ok_or_else(|| {
                        CompileError::Syntax(format!(
                            "pure function `{name}` references unknown struct parameter id {id}"
                        ))
                    })?;
                if variable.starts_with("mut ")
                    || !is_identifier(variable)
                    || !matches!(known.get(variable), Some(StaticType::PureStruct(actual)) if actual == expected)
                {
                    return Err(CompileError::Syntax(format!(
                        "pure function `{name}` argument `{}` must borrow the expected struct local as `&variable`",
                        param.name
                    )));
                }
                variable
            }
        };
        args.push(variable.to_string());
    }
    Ok(args)
}
