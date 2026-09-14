use crate::declarations;
use crate::diagnostics::CompileError;
use crate::module_namespace::{qualify, resolve};
use crate::source_syntax::{function_bounds, is_identifier, matching_brace, split_top_level};
use language_core::{InherentMethod, Program};

/// Register Rust-like inherent impl methods without admitting raw Rust bodies into codegen.
/// Method bodies are lowered separately through the existing verified pure-compute pipeline.
pub(crate) fn register_inherent_impls(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    for block in impl_blocks(source)? {
        if crate::visibility::declaration_visibility(source, block.keyword_pos)
            == language_core::Visibility::Public
        {
            return Err(CompileError::Syntax(
                "`impl` blocks cannot be `pub`; mark individual methods `pub` instead".into(),
            ));
        }
        let target = resolve(namespace, block.target);
        if program.json_schema(&target).is_none() {
            return Err(CompileError::Syntax(format!(
                "inherent impl target `{}` is not a declared struct in this module",
                block.target
            )));
        }
        for method in methods(block.body)? {
            if program.inherent_method(&target, method.name).is_some() {
                return Err(CompileError::Syntax(format!(
                    "duplicate inherent method `{}::{}`",
                    block.target, method.name
                )));
            }
            let function = qualify(
                namespace,
                &format!("__velran_impl_{}_{}", block.target, method.name),
            );
            program.inherent_methods.push(InherentMethod {
                target: target.clone(),
                name: method.name.to_string(),
                function,
                has_receiver: method.has_receiver,
            });
            if method.visibility == language_core::Visibility::Public
                && program.visibility(&target) != language_core::Visibility::Public
            {
                return Err(CompileError::Syntax(format!(
                    "public method `{}::{}` cannot be exposed on private struct `{}`; mark the struct `pub` or keep the method private",
                    block.target, method.name, block.target
                )));
            }
            crate::visibility::register_member(program, &target, method.name, method.visibility);
        }
    }
    Ok(())
}

/// Lower each inherent method into a synthetic ordinary pure function. This deliberately
/// reuses pure-function validation, fuel/allocation accounting and native codegen instead
/// of introducing a second execution path for methods.
pub(crate) fn lower_inherent_impls(
    source: &str,
    namespace: &str,
    program: &mut Program,
) -> Result<(), CompileError> {
    for block in impl_blocks(source)? {
        let target = resolve(namespace, block.target);
        for method in methods(block.body)? {
            let synthetic_name = format!("__velran_impl_{}_{}", block.target, method.name);
            let params = synthetic_params(block.target, method.params, method.has_receiver)?;
            let params = replace_identifier(&params, "Self", block.target);
            let return_suffix = replace_identifier(method.return_suffix, "Self", block.target);
            let body = replace_identifier(method.body, "Self", block.target);
            let body = if method.has_receiver {
                replace_identifier(&body, "self", "self_value")
            } else {
                body
            };
            let synthetic =
                format!("fn {synthetic_name}({params}) {return_suffix} {{\n{body}\n}}\n",);
            crate::pure_functions::parse_pure_functions(&synthetic, namespace, program)?;
            if method.visibility == language_core::Visibility::Public {
                let function_name = qualify(namespace, &synthetic_name);
                let function = program.pure_function(&function_name).ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "internal compiler error: lowered inherent method `{}` has no pure helper",
                        method.name
                    ))
                })?;
                crate::visibility::validate_public_pure_api(
                    &format!("{}::{}", target, method.name),
                    &function.params,
                    &function.return_type,
                    program,
                )?;
            }
        }
    }
    Ok(())
}

struct ImplBlock<'a> {
    target: &'a str,
    body: &'a str,
    keyword_pos: usize,
}

#[derive(Debug)]
struct Method<'a> {
    name: &'a str,
    params: &'a str,
    return_suffix: &'a str,
    body: &'a str,
    has_receiver: bool,
    visibility: language_core::Visibility,
}

fn impl_blocks(source: &str) -> Result<Vec<ImplBlock<'_>>, CompileError> {
    let mut out = Vec::new();
    for (pos, _) in source.match_indices("impl ") {
        if !declarations::is_top_level_declaration_at(source, pos) {
            continue;
        }
        let after = pos + "impl ".len();
        let rest = &source[after..];
        let target_len = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .count();
        if target_len == 0 {
            return Err(CompileError::Syntax("inherent impl target expected".into()));
        }
        let target = &rest[..target_len];
        if !is_identifier(target) {
            return Err(CompileError::Syntax(format!(
                "invalid impl target `{target}`"
            )));
        }
        let tail = &rest[target_len..];
        let trimmed = tail.trim_start();
        if trimmed.starts_with('<') || trimmed.starts_with("for ") {
            return Err(CompileError::Syntax(
                "generic and trait impls are not supported yet; use an inherent `impl Type { ... }`"
                    .into(),
            ));
        }
        let ws = tail.len() - trimmed.len();
        let body_open = after + target_len + ws;
        if source.as_bytes().get(body_open) != Some(&b'{') {
            return Err(CompileError::Syntax(format!(
                "inherent impl `{target}` must use `impl {target} {{ ... }}`"
            )));
        }
        let body_close = matching_brace(source, body_open)
            .ok_or_else(|| CompileError::Syntax(format!("impl `{target}` body unclosed")))?;
        out.push(ImplBlock {
            target,
            body: &source[body_open + 1..body_close],
            keyword_pos: pos,
        });
    }
    Ok(out)
}

fn methods(body: &str) -> Result<Vec<Method<'_>>, CompileError> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while cursor < body.len() {
        cursor = crate::source_syntax::skip_ws_and_comments(body, cursor);
        if cursor >= body.len() {
            break;
        }
        let mut fn_pos = cursor;
        if body[cursor..].starts_with("#[inline") {
            let line_end = body[cursor..]
                .find('\n')
                .map(|v| cursor + v + 1)
                .ok_or_else(|| {
                    CompileError::Syntax("#[inline] in impl must be followed by a method".into())
                })?;
            fn_pos = crate::source_syntax::skip_ws_and_comments(body, line_end);
        }
        let visibility = if body[fn_pos..].starts_with("pub fn ") {
            fn_pos += "pub ".len();
            language_core::Visibility::Public
        } else {
            language_core::Visibility::Private
        };
        if !body[fn_pos..].starts_with("fn ") {
            return Err(CompileError::Syntax(
                "only ordinary `fn` declarations are allowed inside an inherent impl".into(),
            ));
        }
        let start = fn_pos + 3;
        let (name_owned, sig_open, sig_close, body_open, body_close) =
            function_bounds(body, start, "inherent method")?;
        let name_start = start;
        let name = &body[name_start..name_start + name_owned.len()];
        let params = &body[sig_open + 1..sig_close];
        let return_suffix = body[sig_close + 1..body_open].trim();
        let method_body = &body[body_open + 1..body_close];
        let parts = if params.trim().is_empty() {
            Vec::new()
        } else {
            split_top_level(params, ',')
        };
        let has_receiver = parts.first().is_some_and(|v| v.trim() == "&self");
        if parts.iter().any(|v| {
            let v = v.trim();
            v == "self" || v == "&mut self" || v.starts_with("self:")
        }) {
            return Err(CompileError::Syntax(
                "inherent methods currently support only an immutable `&self` receiver; owned or mutable self is rejected"
                    .into(),
            ));
        }
        if parts
            .iter()
            .skip(usize::from(has_receiver))
            .any(|v| v.trim().contains("self"))
        {
            return Err(CompileError::Syntax(
                "`self` is reserved for the receiver inside inherent methods".into(),
            ));
        }
        out.push(Method {
            name,
            params,
            return_suffix,
            body: method_body,
            has_receiver,
            visibility,
        });
        cursor = body_close + 1;
    }
    Ok(out)
}

fn synthetic_params(
    target: &str,
    params: &str,
    has_receiver: bool,
) -> Result<String, CompileError> {
    let parts = if params.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level(params, ',')
    };
    let mut out = Vec::new();
    let start = if has_receiver {
        out.push(format!("self_value: &{target}"));
        1usize
    } else {
        0usize
    };
    for part in parts.into_iter().skip(start) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        out.push(part.to_string());
    }
    Ok(out.join(", "))
}

fn replace_identifier(source: &str, identifier: &str, replacement: &str) -> String {
    debug_assert!(!identifier.is_empty() && identifier.is_ascii());
    let bytes = source.as_bytes();
    let needle = identifier.as_bytes();
    let mut out = Vec::with_capacity(source.len() + replacement.len());
    let mut i = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    while i < bytes.len() {
        if in_string {
            out.push(bytes[i]);
            if escaped {
                escaped = false;
            } else if bytes[i] == b'\\' {
                escaped = true;
            } else if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'"' {
            in_string = true;
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            let mut end = i + 2;
            while end < bytes.len() && bytes[end] != b'\n' {
                end += 1;
            }
            out.extend_from_slice(&bytes[i..end]);
            i = end;
            continue;
        }
        let matches = bytes.get(i..i + needle.len()) == Some(needle);
        if matches {
            let prev_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let next = i + needle.len();
            let next_ok = next >= bytes.len() || !is_ident_byte(bytes[next]);
            if prev_ok && next_ok {
                out.extend_from_slice(replacement.as_bytes());
                i = next;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).expect("identifier rewriting preserves UTF-8")
}

fn is_ident_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_rewrite_does_not_touch_strings_or_longer_identifiers() {
        assert_eq!(
            replace_identifier(
                "return self.value; // self\nlet myself = \"self\";",
                "self",
                "self_value"
            ),
            "return self_value.value; // self\nlet myself = \"self\";"
        );
    }

    #[test]
    fn self_type_rewrite_is_identifier_safe() {
        assert_eq!(
            replace_identifier(
                "fn clone_like(other: &Self) -> Self { return Self { value: self.value }; }",
                "Self",
                "Summary",
            ),
            "fn clone_like(other: &Summary) -> Summary { return Summary { value: self.value }; }"
        );
        assert_eq!(replace_identifier("Selfish", "Self", "Summary"), "Selfish");
    }

    #[test]
    fn mutable_self_is_rejected() {
        let err = methods("fn bump(&mut self) { }").unwrap_err();
        assert!(err.to_string().contains("immutable `&self`"));
    }
}
