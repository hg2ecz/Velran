use crate::declarations;
use crate::diagnostics::CompileError;
use crate::handler_types::StaticType;
use crate::html_template;
use crate::module_namespace::qualify;
use crate::source_syntax::{function_bounds, is_identifier, matching_brace, split_top_level};
use crate::type_resolution::resolve_value_type;
use language_core::{
    ComponentFunction, HtmlPart, HtmlTemplate, LayoutFunction, Program, TemplateParam,
    TemplateParamType, ValueType,
};
use std::collections::{HashMap, HashSet};

fn parse_template_param_type(
    raw: &str,
    namespace: &str,
    p: &Program,
) -> Result<TemplateParamType, CompileError> {
    let raw = raw.trim();
    if let Some(v) = resolve_value_type(raw, namespace, p) {
        if matches!(
            v,
            ValueType::Upload | ValueType::F32Array | ValueType::StringList | ValueType::StringDict
        ) {
            return Err(CompileError::Syntax(
                "Upload is not allowed as component/layout parameter".into(),
            ));
        }
        return Ok(TemplateParamType::Scalar(v));
    }
    if let Some(base) = raw.strip_suffix('?') {
        if p.model(&crate::module_namespace::resolve(namespace, base))
            .is_some()
        {
            return Ok(TemplateParamType::OptionalModel(
                crate::module_namespace::resolve(namespace, base),
            ));
        }
    }
    if raw.starts_with("Vec<") && raw.ends_with('>') {
        let base = &raw[5..raw.len() - 1];
        if p.model(&crate::module_namespace::resolve(namespace, base))
            .is_some()
        {
            return Ok(TemplateParamType::ListModel(
                crate::module_namespace::resolve(namespace, base),
            ));
        }
    }
    let model_name = crate::module_namespace::resolve(namespace, raw);
    if p.model(&model_name).is_some() {
        return Ok(TemplateParamType::Model(model_name));
    }
    Err(CompileError::Syntax(format!(
        "unknown component/layout parameter type `{raw}`"
    )))
}
pub(super) fn template_static_type(t: &TemplateParamType) -> StaticType {
    match t {
        TemplateParamType::Scalar(v) => StaticType::trusted_scalar(*v),
        TemplateParamType::Model(v) => StaticType::model(v.clone()),
        TemplateParamType::OptionalModel(v) => StaticType::OptionalModel(v.clone()),
        TemplateParamType::ListModel(v) => StaticType::ListModel(v.clone()),
    }
}

pub(super) fn argument_type_compatible(
    actual: &StaticType,
    expected: &TemplateParamType,
    program: &Program,
) -> bool {
    match (actual, expected) {
        (StaticType::Scalar(actual), TemplateParamType::Scalar(expected)) => {
            if matches!(expected, ValueType::Domain(_) | ValueType::Credential(_)) {
                actual.value_type == *expected
            } else {
                program.representation_type(actual.value_type) == Some(*expected)
            }
        }
        (StaticType::Model(actual), TemplateParamType::Model(expected)) => actual.name == *expected,
        (StaticType::OptionalModel(actual), TemplateParamType::OptionalModel(expected)) => {
            actual == expected
        }
        (StaticType::ListModel(actual), TemplateParamType::ListModel(expected)) => {
            actual == expected
        }
        _ => false,
    }
}

fn parse_template_params(
    input: &str,
    namespace: &str,
    p: &Program,
    name: &str,
) -> Result<Vec<TemplateParam>, CompileError> {
    let mut out = Vec::new();
    for raw in split_top_level(input, ',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (n, t) = raw.split_once(':').ok_or_else(|| {
            CompileError::Syntax(format!(
                "template `{name}` parameter `{raw}` expected `name: Type`"
            ))
        })?;
        let n = n.trim();
        if !is_identifier(n) {
            return Err(CompileError::Syntax(format!(
                "template `{name}` invalid parameter `{n}`"
            )));
        }
        if out.iter().any(|x: &TemplateParam| x.name == n) {
            return Err(CompileError::Syntax(format!(
                "template `{name}` duplicate parameter `{n}`"
            )));
        }
        out.push(TemplateParam {
            name: n.into(),
            ty: parse_template_param_type(t, namespace, p)?,
        });
    }
    Ok(out)
}
fn extract_html_body<'a>(kind: &str, name: &str, body: &'a str) -> Result<&'a str, CompileError> {
    let trimmed = body.trim();
    let rest = trimmed.strip_prefix("html").ok_or_else(|| {
        CompileError::Syntax(format!("{kind} `{name}` body must be `html {{ ... }}`"))
    })?;
    let rel = rest
        .find('{')
        .ok_or_else(|| CompileError::Syntax(format!("{kind} `{name}` html body missing `{{`")))?;
    let open = trimmed.len() - rest.len() + rel;
    let close = matching_brace(trimmed, open)
        .ok_or_else(|| CompileError::Syntax(format!("{kind} `{name}` html body unclosed")))?;
    if !trimmed[close + 1..].trim().is_empty() {
        return Err(CompileError::Syntax(format!(
            "{kind} `{name}` has content after html body"
        )));
    }
    Ok(&trimmed[open + 1..close])
}
pub(super) fn parse_template_functions(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    #[derive(Clone)]
    struct Pending {
        kind: u8,
        name: String,
        params: Vec<TemplateParam>,
        body: String,
    }
    let mut pending = Vec::new();
    for (needle, kind) in [("component fn ", 0u8), ("layout fn ", 1u8)] {
        let mut off = 0usize;
        while let Some(rel) = source[off..].find(needle) {
            let keyword = off + rel;
            if !declarations::is_top_level_declaration_at(source, keyword) {
                off = keyword + needle.len();
                continue;
            }
            let start = keyword + needle.len();
            let (name, sig_open, sig_close, body_open, body_close) = function_bounds(
                source,
                start,
                if kind == 0 { "component" } else { "layout" },
            )?;
            let tail: String = source[sig_close + 1..body_open]
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            if tail != "->Html" {
                return Err(CompileError::Syntax(format!(
                    "{} `{name}` must return Html",
                    if kind == 0 { "component" } else { "layout" }
                )));
            }
            let symbol_name = qualify(namespace, &name);
            if p.component(&symbol_name).is_some() || p.layout(&symbol_name).is_some() {
                return Err(CompileError::Syntax(format!("duplicate template `{name}`")));
            }
            let params =
                parse_template_params(&source[sig_open + 1..sig_close], namespace, p, &name)?;
            pending.push(Pending {
                kind,
                name: symbol_name.clone(),
                params: params.clone(),
                body: extract_html_body(
                    if kind == 0 { "component" } else { "layout" },
                    &name,
                    &source[body_open + 1..body_close],
                )?
                .into(),
            });
            if kind == 0 {
                p.components.push(ComponentFunction {
                    name: symbol_name,
                    params,
                    template: HtmlTemplate { parts: Vec::new() },
                });
            } else {
                p.layouts.push(LayoutFunction {
                    name: symbol_name,
                    params,
                    template: HtmlTemplate { parts: Vec::new() },
                });
            }
            off = body_close + 1;
        }
    }
    for item in pending {
        let known: HashMap<String, StaticType> = item
            .params
            .iter()
            .map(|x| (x.name.clone(), template_static_type(&x.ty)))
            .collect();
        let template = html_template::parse_html_template_mode(
            &item.body,
            namespace,
            &known,
            p,
            item.kind == 1,
        )?;
        if item.kind == 1 {
            let slots = count_content_slots(&template);
            if slots != 1 {
                return Err(CompileError::Syntax(format!(
                    "layout `{}` must contain exactly one @content slot",
                    item.name
                )));
            }
            let idx = p.layouts.iter().position(|x| x.name == item.name).unwrap();
            p.layouts[idx].template = template;
        } else {
            if count_content_slots(&template) != 0 {
                return Err(CompileError::Syntax(format!(
                    "component `{}` cannot contain @content",
                    item.name
                )));
            }
            let idx = p
                .components
                .iter()
                .position(|x| x.name == item.name)
                .unwrap();
            p.components[idx].template = template;
        }
    }
    Ok(())
}
fn count_content_slots(t: &HtmlTemplate) -> usize {
    t.parts
        .iter()
        .map(|p| match p {
            HtmlPart::ContentSlot => 1,
            HtmlPart::For { template, .. } | HtmlPart::IfSome { template, .. } => {
                count_content_slots(template)
            }
            HtmlPart::LayoutCall { content, .. } => count_content_slots(content),
            _ => 0,
        })
        .sum()
}

fn template_edges(t: &HtmlTemplate, out: &mut Vec<String>) {
    for part in &t.parts {
        match part {
            HtmlPart::ComponentCall { component, .. } => out.push(format!("c:{component}")),
            HtmlPart::LayoutCall {
                layout, content, ..
            } => {
                out.push(format!("l:{layout}"));
                template_edges(content, out)
            }
            HtmlPart::For { template, .. } | HtmlPart::IfSome { template, .. } => {
                template_edges(template, out)
            }
            _ => {}
        }
    }
}
pub(super) fn validate_template_cycles(p: &Program) -> Result<(), CompileError> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for c in &p.components {
        let mut e = Vec::new();
        template_edges(&c.template, &mut e);
        graph.insert(format!("c:{}", c.name), e);
    }
    for l in &p.layouts {
        let mut e = Vec::new();
        template_edges(&l.template, &mut e);
        graph.insert(format!("l:{}", l.name), e);
    }
    fn visit(
        node: &str,
        g: &HashMap<String, Vec<String>>,
        temp: &mut HashSet<String>,
        done: &mut HashSet<String>,
    ) -> Result<(), CompileError> {
        if done.contains(node) {
            return Ok(());
        }
        if !temp.insert(node.into()) {
            return Err(CompileError::Syntax(format!(
                "component/layout cycle detected at `{}`",
                &node[2..]
            )));
        }
        if let Some(edges) = g.get(node) {
            for e in edges {
                visit(e, g, temp, done)?;
            }
        }
        temp.remove(node);
        done.insert(node.into());
        Ok(())
    }
    let mut done = HashSet::new();
    for node in graph.keys() {
        let mut temp = HashSet::new();
        visit(node, &graph, &mut temp, &mut done)?;
    }
    Ok(())
}
