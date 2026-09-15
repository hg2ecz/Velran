use crate::diagnostics::CompileError;
use crate::domain_symbols::internal_domain_symbol;
use crate::expression::{
    infer_expr_type, infer_static_expr_type, parse_expr_in_namespace, validate_expr,
};
use crate::handler_types::StaticType;
use crate::module_namespace::resolve;
use crate::response_security::{ResponseBoundary, validate_response_expression};
use crate::source_syntax::{
    is_identifier, matching_brace, matching_paren, skip_ws_and_comments, split_top_level,
};
use crate::type_semantics::represented_as;
use language_core::{
    HtmlAttrKind, HtmlPart, HtmlTemplate, HttpMethod, Program, RouteSegment, ValueType,
};
use std::collections::HashMap;

pub(super) fn parse_html_template(
    input: &str,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<HtmlTemplate, CompileError> {
    parse_html_template_mode(input, namespace, known, p, false)
}
pub(super) fn parse_html_template_mode(
    input: &str,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    p: &Program,
    allow_content: bool,
) -> Result<HtmlTemplate, CompileError> {
    validate_static_html(input)?;
    validate_interpolation_contexts(input)?;
    validate_known_template_directives(input)?;
    validate_template_directive_contexts(input)?;
    parse_html_parts_mode(input, namespace, known, p, allow_content)
}

fn parse_html_parts_mode(
    input: &str,
    namespace: &str,
    known: &HashMap<String, StaticType>,
    p: &Program,
    allow_content: bool,
) -> Result<HtmlTemplate, CompileError> {
    let mut parts = Vec::new();
    let mut cursor = 0;
    while cursor < input.len() {
        let interp = input[cursor..].find("{{").map(|v| cursor + v);
        let for_pos = input[cursor..].find("@for ").map(|v| cursor + v);
        let if_pos = input[cursor..].find("@if ").map(|v| cursor + v);
        let href_pos = input[cursor..].find("@href(").map(|v| cursor + v);
        let action_pos = input[cursor..].find("@action(").map(|v| cursor + v);
        let component_pos = input[cursor..].find("@component(").map(|v| cursor + v);
        let layout_pos = input[cursor..].find("@layout(").map(|v| cursor + v);
        let content_pos = input[cursor..].find("@content").map(|v| cursor + v);
        let image_pos = input[cursor..].find("@image(").map(|v| cursor + v);
        let flash_pos = input[cursor..].find("@flash()").map(|v| cursor + v);
        let next = [
            interp,
            for_pos,
            if_pos,
            href_pos,
            action_pos,
            component_pos,
            layout_pos,
            content_pos,
            image_pos,
            flash_pos,
        ]
        .into_iter()
        .flatten()
        .min();
        let Some(pos) = next else {
            if cursor < input.len() {
                parts.push(HtmlPart::Text(input[cursor..].into()));
            }
            break;
        };
        if pos > cursor {
            parts.push(HtmlPart::Text(input[cursor..pos].into()));
        }
        if Some(pos) == interp {
            let s = pos + 2;
            let close = input[s..]
                .find("}}")
                .map(|v| s + v)
                .ok_or_else(|| CompileError::Syntax("unclosed HTML interpolation".into()))?;
            let e = parse_expr_in_namespace(input[s..close].trim(), namespace, p)?;
            validate_expr(&e, known, p)?;
            crate::visibility::validate_pure_expr_access(&e, known, p, namespace)?;
            validate_response_expression(&e, known, p, ResponseBoundary::Html)?;
            let interpolation_type = infer_expr_type(&e, known, p)?;
            if interpolation_type == ValueType::Image {
                return Err(CompileError::Syntax(
                    "Image cannot be interpolated directly; use @image(image, alt)".into(),
                ));
            }
            if interpolation_type == ValueType::Domain(language_core::SAFE_HTML_DOMAIN_ID) {
                parts.push(HtmlPart::SafeHtmlExpr(e));
            } else {
                parts.push(HtmlPart::EscapedExpr(e));
            }
            cursor = close + 2;
            continue;
        }
        if Some(pos) == content_pos {
            if !allow_content {
                return Err(CompileError::Syntax(
                    "@content is only allowed inside layout definitions".into(),
                ));
            }
            parts.push(HtmlPart::ContentSlot);
            cursor = pos + "@content".len();
            continue;
        }
        if Some(pos) == flash_pos {
            parts.push(HtmlPart::Flash);
            cursor = pos + "@flash()".len();
            continue;
        }
        if Some(pos) == image_pos {
            let open = pos + "@image(".len() - 1;
            let close = matching_paren(input, open)
                .ok_or_else(|| CompileError::Syntax("@image( is not closed".into()))?;
            let raw = split_top_level(&input[open + 1..close], ',');
            if raw.len() != 2 {
                return Err(CompileError::Syntax(
                    "@image requires Image and alt String expressions".into(),
                ));
            }
            let image = parse_expr_in_namespace(raw[0].trim(), namespace, p)?;
            let alt = parse_expr_in_namespace(raw[1].trim(), namespace, p)?;
            validate_expr(&image, known, p)?;
            crate::visibility::validate_pure_expr_access(&image, known, p, namespace)?;
            validate_expr(&alt, known, p)?;
            crate::visibility::validate_pure_expr_access(&alt, known, p, namespace)?;
            validate_response_expression(&image, known, p, ResponseBoundary::Html)?;
            validate_response_expression(&alt, known, p, ResponseBoundary::Html)?;
            if infer_expr_type(&image, known, p)? != ValueType::Image {
                return Err(CompileError::Syntax(
                    "@image first expression must have type Image".into(),
                ));
            }
            if !represented_as(p, infer_expr_type(&alt, known, p)?, ValueType::String) {
                return Err(CompileError::Syntax(
                    "@image alt expression must have String representation".into(),
                ));
            }
            parts.push(HtmlPart::Image { image, alt });
            cursor = close + 1;
            continue;
        }
        if Some(pos) == component_pos || Some(pos) == layout_pos {
            let is_layout = Some(pos) == layout_pos;
            let prefix = if is_layout { "@layout(" } else { "@component(" };
            let open = pos + prefix.len() - 1;
            let close = matching_paren(input, open)
                .ok_or_else(|| CompileError::Syntax(format!("{prefix} is not closed")))?;
            let raw = split_top_level(&input[open + 1..close], ',');
            if raw.is_empty() {
                return Err(CompileError::Syntax(format!(
                    "{prefix} requires a template name"
                )));
            }
            let source_name = raw[0].trim();
            let name = internal_domain_symbol(source_name)
                .map(|name| resolve(namespace, &name))
                .ok_or_else(|| {
                    CompileError::Syntax(
                        "template call requires a component/layout name as first argument".into(),
                    )
                })?;
            let params = if is_layout {
                p.layout(&name).map(|x| &x.params)
            } else {
                p.component(&name).map(|x| &x.params)
            }
            .ok_or_else(|| {
                CompileError::Syntax(format!(
                    "unknown {} `{source_name}`",
                    if is_layout { "layout" } else { "component" }
                ))
            })?;
            if raw.len() - 1 != params.len() {
                return Err(CompileError::Syntax(format!(
                    "template `{name}` expects {} arguments, got {}",
                    params.len(),
                    raw.len() - 1
                )));
            }
            let mut args = Vec::new();
            for (piece, param) in raw.iter().skip(1).zip(params.iter()) {
                let e = parse_expr_in_namespace(piece.trim(), namespace, p)?;
                validate_expr(&e, known, p)?;
                crate::visibility::validate_pure_expr_access(&e, known, p, namespace)?;
                let actual = infer_static_expr_type(&e, known, p)?;
                if !super::template_declarations::argument_type_compatible(&actual, &param.ty, p) {
                    return Err(CompileError::Syntax(format!(
                        "template `{name}` argument `{}` type mismatch",
                        param.name
                    )));
                }
                if actual.scalar().is_some() {
                    validate_response_expression(&e, known, p, ResponseBoundary::Html)?;
                }
                args.push(e);
            }
            if is_layout {
                let j = skip_ws_and_comments(input, close + 1);
                if input.as_bytes().get(j) != Some(&b'{') {
                    return Err(CompileError::Syntax(format!(
                        "@layout `{source_name}` requires a content block"
                    )));
                }
                let end = matching_brace(input, j).ok_or_else(|| {
                    CompileError::Syntax(format!("@layout `{source_name}` content block unclosed"))
                })?;
                let content =
                    parse_html_parts_mode(&input[j + 1..end], namespace, known, p, false)?;
                parts.push(HtmlPart::LayoutCall {
                    layout: name.clone(),
                    args,
                    content,
                });
                cursor = end + 1;
            } else {
                parts.push(HtmlPart::ComponentCall {
                    component: name.clone(),
                    args,
                });
                cursor = close + 1;
            }
            continue;
        }
        if Some(pos) == href_pos || Some(pos) == action_pos {
            let (kind, prefix) = if Some(pos) == href_pos {
                (HtmlAttrKind::Href, "@href(")
            } else {
                (HtmlAttrKind::Action, "@action(")
            };
            let open = pos + prefix.len() - 1;
            let close = matching_paren(input, open)
                .ok_or_else(|| CompileError::Syntax(format!("{prefix} is not closed")))?;
            let raw = split_top_level(&input[open + 1..close], ',');
            if raw.is_empty() {
                return Err(CompileError::Syntax(format!(
                    "{prefix} requires a route name"
                )));
            }
            let route_name = raw[0].trim();
            if !is_identifier(route_name) {
                return Err(CompileError::Syntax(
                    "typed URL helper requires a route identifier as first argument".into(),
                ));
            }
            let route = p
                .routes
                .iter()
                .find(|r| r.name == route_name)
                .ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "typed URL helper references unknown route `{route_name}`"
                    ))
                })?;
            match kind {
                HtmlAttrKind::Href if route.method != HttpMethod::Get => {
                    return Err(CompileError::UnsafeHtml(format!(
                        "@href requires GET route; `{route_name}` is POST"
                    )));
                }
                HtmlAttrKind::Action if route.method != HttpMethod::Post => {
                    return Err(CompileError::UnsafeHtml(format!(
                        "@action requires POST route; `{route_name}` is GET"
                    )));
                }
                _ => {}
            }
            let mut expected: Vec<ValueType> = route
                .segments
                .iter()
                .filter_map(|s| match s {
                    RouteSegment::Param { ty, .. } => Some(*ty),
                    _ => None,
                })
                .collect();
            if matches!(kind, HtmlAttrKind::Href) {
                expected.extend(route.query_fields.iter().map(|f| f.ty));
            }
            if raw.len() - 1 != expected.len() {
                return Err(CompileError::Syntax(format!(
                    "typed URL helper for `{route_name}` expects {} arguments, got {}",
                    expected.len(),
                    raw.len() - 1
                )));
            }
            let mut args = Vec::new();
            for (piece, ty) in raw.iter().skip(1).zip(expected) {
                let e = parse_expr_in_namespace(piece.trim(), namespace, p)?;
                validate_expr(&e, known, p)?;
                crate::visibility::validate_pure_expr_access(&e, known, p, namespace)?;
                if infer_expr_type(&e, known, p)? != ty {
                    return Err(CompileError::Syntax(format!(
                        "typed URL helper argument type mismatch for route `{route_name}`"
                    )));
                }
                args.push(e);
            }
            parts.push(HtmlPart::RouteAttr {
                kind,
                route: route_name.into(),
                args,
            });
            cursor = close + 1;
            continue;
        }
        if Some(pos) == for_pos {
            let header_start = pos + 5;
            let open = input[header_start..]
                .find('{')
                .map(|v| header_start + v)
                .ok_or_else(|| CompileError::Syntax("@for missing `{`".into()))?;
            let header = input[header_start..open].trim();
            let (item, collection) = header.split_once(" in ").ok_or_else(|| {
                CompileError::Syntax("@for syntax is `@for item in items { ... }`".into())
            })?;
            let item = item.trim();
            let collection = collection.trim();
            if !is_identifier(item) || !is_identifier(collection) {
                return Err(CompileError::Syntax("@for identifiers are invalid".into()));
            }
            let model = match known.get(collection) {
                Some(StaticType::ListModel(m)) => m.clone(),
                _ => {
                    return Err(CompileError::Syntax(format!(
                        "@for collection `{collection}` is not `List<Model>`"
                    )));
                }
            };
            let close = matching_brace(input, open)
                .ok_or_else(|| CompileError::Syntax("@for body unclosed".into()))?;
            let mut nested = known.clone();
            nested.insert(item.into(), StaticType::model(model));
            let template =
                parse_html_parts_mode(&input[open + 1..close], namespace, &nested, p, false)?;
            parts.push(HtmlPart::For {
                item: item.into(),
                collection: collection.into(),
                template,
            });
            cursor = close + 1;
            continue;
        }
        let header_start = pos + 4;
        let open = input[header_start..]
            .find('{')
            .map(|v| header_start + v)
            .ok_or_else(|| CompileError::Syntax("@if missing `{`".into()))?;
        let value = input[header_start..open].trim();
        if !is_identifier(value) {
            return Err(CompileError::Syntax(
                "@if expects an optional model variable".into(),
            ));
        }
        let model = match known.get(value) {
            Some(StaticType::OptionalModel(m)) => m.clone(),
            _ => {
                return Err(CompileError::Syntax(format!(
                    "@if `{value}` requires `Model?`"
                )));
            }
        };
        let close = matching_brace(input, open)
            .ok_or_else(|| CompileError::Syntax("@if body unclosed".into()))?;
        let mut nested = known.clone();
        nested.insert(value.into(), StaticType::model(model));
        let template =
            parse_html_parts_mode(&input[open + 1..close], namespace, &nested, p, false)?;
        parts.push(HtmlPart::IfSome {
            value: value.into(),
            template,
        });
        cursor = close + 1;
    }
    Ok(HtmlTemplate { parts })
}

fn validate_known_template_directives(input: &str) -> Result<(), CompileError> {
    const KNOWN_CALL_DIRECTIVES: &[&str] =
        &["href", "action", "component", "layout", "image", "flash"];

    let bytes = input.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let Some(rel) = input[cursor..].find('@') else {
            break;
        };
        let at = cursor + rel;
        let ident_start = at + 1;
        if ident_start >= bytes.len()
            || !(bytes[ident_start] == b'_' || bytes[ident_start].is_ascii_alphabetic())
        {
            cursor = ident_start;
            continue;
        }

        let mut ident_end = ident_start + 1;
        while ident_end < bytes.len()
            && (bytes[ident_end] == b'_' || bytes[ident_end].is_ascii_alphanumeric())
        {
            ident_end += 1;
        }

        if bytes.get(ident_end) == Some(&b'(') {
            let name = &input[ident_start..ident_end];
            if !KNOWN_CALL_DIRECTIVES.contains(&name) {
                return Err(CompileError::Syntax(format!(
                    "unknown HTML template directive `@{name}`"
                )));
            }
        }

        cursor = ident_end;
    }
    Ok(())
}

fn validate_template_directive_contexts(input: &str) -> Result<(), CompileError> {
    for needle in ["@component(", "@layout(", "@content", "@image(", "@flash()"] {
        let mut cursor = 0usize;
        while let Some(rel) = input[cursor..].find(needle) {
            let pos = cursor + rel;
            let lt = input[..pos].rfind('<');
            let gt = input[..pos].rfind('>');
            if lt.is_some() && lt > gt {
                return Err(CompileError::UnsafeHtml(format!(
                    "{needle} is only allowed in HTML content, not inside a tag/attribute"
                )));
            }
            cursor = pos + needle.len();
        }
    }
    Ok(())
}

fn validate_interpolation_contexts(input: &str) -> Result<(), CompileError> {
    let mut cursor = 0usize;
    while let Some(rel) = input[cursor..].find("{{") {
        let pos = cursor + rel;
        let lt = input[..pos].rfind('<');
        let gt = input[..pos].rfind('>');
        if lt.is_some() && lt > gt {
            let tag = &input[lt.unwrap()..pos];
            let compact = tag.trim_end();
            let allowed = compact.ends_with("value=\"") || compact.ends_with("value='");
            if !allowed {
                return Err(CompileError::UnsafeHtml("dynamic interpolation inside an HTML tag is forbidden in v0.1 except quoted `value=\"{{ ... }}\"`; use a future typed URL/attribute helper".into()));
            }
            let close = input[pos + 2..]
                .find("}}")
                .map(|v| pos + 2 + v)
                .ok_or_else(|| CompileError::Syntax("unclosed HTML interpolation".into()))?;
            let after = &input[close + 2..];
            let quote = if compact.ends_with('"') { '"' } else { '\'' };
            if !after.starts_with(quote) {
                return Err(CompileError::UnsafeHtml(
                    "attribute interpolation must occupy the complete quoted attribute value"
                        .into(),
                ));
            }
        }
        cursor = pos + 2;
    }
    Ok(())
}
fn validate_static_html(input: &str) -> Result<(), CompileError> {
    let lower = input.to_ascii_lowercase();
    for x in [
        "<script",
        "</script",
        "<style",
        "</style",
        "javascript:",
        "<iframe",
        "<object",
        "<embed",
        "<base",
        " style=",
    ] {
        if lower.contains(x) {
            return Err(CompileError::UnsafeHtml(format!("`{x}` is forbidden")));
        }
    }
    let b = lower.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_whitespace() && b.get(i + 1) == Some(&b'o') && b.get(i + 2) == Some(&b'n')
        {
            let mut j = i + 3;
            while j < b.len() && b[j].is_ascii_alphabetic() {
                j += 1;
            }
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            if b.get(j) == Some(&b'=') {
                return Err(CompileError::UnsafeHtml(
                    "inline event handlers are forbidden".into(),
                ));
            }
        }
        i += 1;
    }
    Ok(())
}
