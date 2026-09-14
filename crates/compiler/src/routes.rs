use crate::cache_safety::{
    action_has_business_audit, action_has_object_auth, validate_public_cache_statements,
};
use crate::diagnostics::CompileError;
use crate::domain_symbols::internal_domain_symbol;
use crate::domain_validation;
use crate::lexer::tokenize;
use crate::module_namespace::resolve;
use crate::route_security::{apply_default_external_input_bounds, parse_explicit_access};
use crate::source_syntax::is_identifier;
use crate::type_resolution::resolve_value_type;
use language_core::{
    ActionBody, FormField, HttpMethod, PageBody, Program, Route, RouteAuth, RouteSegment,
    ValidationKind, ValueType,
};
use std::collections::HashMap;

mod route_scanner;

pub(super) fn parse_routes(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    for declaration in route_scanner::top_level_route_declarations(source)? {
        let route_source = declaration.source;
        let t = tokenize(&route_source)?;
        let i = 0;
        if t.get(i).map(String::as_str) != Some("route") {
            return Err(CompileError::Syntax(
                "internal route scanner error: declaration does not start with `route`".into(),
            ));
        }
        if i + 5 >= t.len() {
            return Err(CompileError::Syntax("incomplete route".into()));
        }
        let name = t[i + 1].clone();
        let method = HttpMethod::parse(&t[i + 2])
            .ok_or_else(|| CompileError::Syntax("unsupported route method".into()))?;
        let path = t[i + 3].clone();
        let (segments, mut validations) = parse_route_segments(&name, &path, namespace, p)?;
        let mut c = i + 4;
        let mut query_fields = Vec::new();
        let mut form_fields = Vec::new();
        let mut json_fields = Vec::new();
        let mut multipart_fields = Vec::new();
        let mut multipart_schema = None;
        let mut upload = None;
        if t.get(c).map(String::as_str) == Some("query") {
            c += 1;
            let named_schema = if let Some(token) = t.get(c) {
                if !token.contains('<') {
                    query_fields =
                        crate::route_typed_schema::query_fields(&name, token, namespace, p)?;
                    c += 1;
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !named_schema {
                while !matches!(
                    t.get(c).map(String::as_str),
                    Some("form")
                        | Some("json")
                        | Some("upload")
                        | Some("multipart")
                        | Some("validate")
                        | Some("tenant")
                        | Some("webhook")
                        | Some("public")
                        | Some("auth")
                        | Some("rate")
                        | Some("budget")
                        | Some("idempotent")
                        | Some("cache")
                        | Some("invalidate")
                        | Some("=>")
                        | None
                ) {
                    let raw = t.get(c).unwrap();
                    let field = parse_typed_binding(&name, raw, namespace, p)?;
                    validations.extend(domain_validation::rules_for_binding(
                        raw,
                        &field.name,
                        namespace,
                        p,
                    ));
                    query_fields.push(field);
                    c += 1;
                }
            }
        }
        let mut form_schema = None;
        if t.get(c).map(String::as_str) == Some("form") {
            c += 1;
            if let Some(token) = t.get(c) {
                if !token.contains('<') {
                    let (fields, schema_validations, legacy_schema) =
                        crate::route_typed_schema::form_fields(&name, token, namespace, p)?;
                    form_fields = fields;
                    if legacy_schema.is_some() {
                        validations = schema_validations;
                    }
                    form_schema = legacy_schema;
                    c += 1;
                } else {
                    while !matches!(
                        t.get(c).map(String::as_str),
                        Some("json")
                            | Some("upload")
                            | Some("multipart")
                            | Some("validate")
                            | Some("tenant")
                            | Some("webhook")
                            | Some("public")
                            | Some("auth")
                            | Some("rate")
                            | Some("budget")
                            | Some("idempotent")
                            | Some("cache")
                            | Some("invalidate")
                            | Some("=>")
                            | None
                    ) {
                        let raw = t.get(c).unwrap();
                        let field = parse_typed_binding(&name, raw, namespace, p)?;
                        validations.extend(domain_validation::rules_for_binding(
                            raw,
                            &field.name,
                            namespace,
                            p,
                        ));
                        form_fields.push(field);
                        c += 1;
                    }
                }
            }
        }
        if t.get(c).map(String::as_str) == Some("json") {
            c += 1;
            let named_schema = if let Some(token) = t.get(c) {
                if !token.contains('<') {
                    let schema_name = resolve(namespace, token);
                    if let Some(schema) = p.json_schema(&schema_name) {
                        if schema
                            .fields
                            .iter()
                            .any(|f| matches!(f.ty, ValueType::Upload | ValueType::Image))
                        {
                            return Err(CompileError::Syntax(format!(
                                "route `{name}` json struct `{token}` cannot contain Upload/Image fields"
                            )));
                        }
                        json_fields = schema.fields.clone();
                        c += 1;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };
            if !named_schema {
                while !matches!(
                    t.get(c).map(String::as_str),
                    Some("upload")
                        | Some("multipart")
                        | Some("validate")
                        | Some("tenant")
                        | Some("webhook")
                        | Some("public")
                        | Some("auth")
                        | Some("rate")
                        | Some("budget")
                        | Some("idempotent")
                        | Some("cache")
                        | Some("invalidate")
                        | Some("=>")
                        | None
                ) {
                    let raw = t.get(c).unwrap();
                    let field = parse_typed_binding(&name, raw, namespace, p)?;
                    validations.extend(domain_validation::rules_for_binding(
                        raw,
                        &field.name,
                        namespace,
                        p,
                    ));
                    json_fields.push(field);
                    c += 1;
                }
            }
            if json_fields.is_empty() {
                return Err(CompileError::Syntax(format!(
                    "route `{name}` json body requires at least one typed field"
                )));
            }
        }
        if t.get(c).map(String::as_str) == Some("multipart") {
            let (fields, typed_upload, schema_name) =
                crate::route_multipart::parse(&t, &mut c, &name, namespace, p)?;
            multipart_fields = fields;
            multipart_schema = Some(schema_name);
            upload = Some(typed_upload);
        }
        if t.get(c).map(String::as_str) == Some("upload") {
            upload = Some(crate::route_upload::parse_legacy(&t, &mut c)?);
        }
        if t.get(c).map(String::as_str) == Some("validate") {
            c += 1;
            while !matches!(
                t.get(c).map(String::as_str),
                Some("tenant")
                    | Some("webhook")
                    | Some("public")
                    | Some("auth")
                    | Some("rate")
                    | Some("budget")
                    | Some("idempotent")
                    | Some("cache")
                    | Some("invalidate")
                    | Some("=>")
                    | None
            ) {
                let (rule, next) = crate::validation_rules::parse_validation_rule(&t, c)?;
                validations.push(rule);
                c = next;
            }
        }
        let tenant_field = crate::route_tenant::parse_tenant_field(&t, &mut c, &name)?;
        let auth = parse_explicit_access(&name, &t, &mut c, namespace, p)?;
        let mut rate_policy = None;
        if t.get(c).map(String::as_str) == Some("rate") {
            let policy = t
                .get(c + 1)
                .ok_or_else(|| {
                    CompileError::Syntax(format!("route `{name}` rate policy expected"))
                })?
                .clone();
            if !is_identifier(&policy) {
                return Err(CompileError::Syntax(format!(
                    "route `{name}` invalid rate policy name"
                )));
            }
            rate_policy = Some(policy);
            c += 2;
        }
        let budget_profile = crate::route_budget::parse_route_budget(
            &t,
            &mut c,
            &name,
            namespace,
            declaration.line,
            p,
        )?;
        let idempotent = if t.get(c).map(String::as_str) == Some("idempotent") {
            c += 1;
            true
        } else {
            false
        };
        let public_cache = crate::route_cache::parse_public_cache(&t, &mut c, &name)?;
        let invalidate_caches = crate::route_cache::parse_invalidations(&t, &mut c, &name)?;
        if t.get(c).map(String::as_str) != Some("=>") {
            return Err(CompileError::Syntax(format!("route `{name}` expected =>")));
        }
        let source_handler = t
            .get(c + 1)
            .ok_or_else(|| CompileError::Syntax("route missing handler".into()))?;
        if t.get(c + 2).map(String::as_str) != Some(";") || t.get(c + 3).is_some() {
            return Err(CompileError::Syntax(format!(
                "route `{name}` must end with `;` immediately after the handler"
            )));
        }
        let handler = internal_domain_symbol(source_handler)
            .map(|name| resolve(namespace, &name))
            .ok_or_else(|| {
                CompileError::Syntax(format!("route `{name}` invalid handler `{source_handler}`"))
            })?;
        if method == HttpMethod::Get
            && (!form_fields.is_empty()
                || !json_fields.is_empty()
                || multipart_schema.is_some()
                || (upload.is_some() && multipart_schema.is_none()))
        {
            return Err(CompileError::Syntax(
                "GET route cannot declare form/json/multipart/upload".into(),
            ));
        }
        if method == HttpMethod::Post && !query_fields.is_empty() {
            return Err(CompileError::Syntax(
                "POST route query schema is not supported".into(),
            ));
        }
        if [
            !form_fields.is_empty(),
            !json_fields.is_empty(),
            multipart_schema.is_some(),
            upload.is_some() && multipart_schema.is_none(),
        ]
        .into_iter()
        .filter(|v| *v)
        .count()
            > 1
        {
            return Err(CompileError::Syntax(
                "POST route may declare exactly one body mode: form, json, multipart, or upload"
                    .into(),
            ));
        }
        let path_fields: Vec<FormField> = segments
            .iter()
            .filter_map(|segment| match segment {
                RouteSegment::Param { name, ty } => Some(FormField {
                    name: name.clone(),
                    ty: *ty,
                }),
                RouteSegment::Static(_) => None,
            })
            .collect();
        let mut field_types = HashMap::new();
        for f in path_fields
            .iter()
            .chain(query_fields.iter())
            .chain(form_fields.iter())
            .chain(json_fields.iter())
            .chain(multipart_fields.iter())
        {
            if field_types.insert(f.name.clone(), f.ty).is_some() {
                return Err(CompileError::Syntax(format!(
                    "route `{name}` duplicate input field `{}`",
                    f.name
                )));
            }
        }
        let tenant_input_types: HashMap<String, ValueType> = path_fields
            .iter()
            .chain(query_fields.iter())
            .map(|field| (field.name.clone(), field.ty))
            .collect();
        crate::route_tenant::validate_tenant_field(
            &name,
            tenant_field.as_deref(),
            &tenant_input_types,
            &auth,
            p,
        )?;
        for v in &validations {
            let ty = *field_types.get(&v.field).ok_or_else(|| {
                CompileError::Syntax(format!(
                    "validation references unknown route input field `{}`",
                    v.field
                ))
            })?;
            let representation = p.representation_type(ty).unwrap_or(ty);
            match &v.kind {
                ValidationKind::Length { .. } if representation == ValueType::String => {}
                ValidationKind::Range { .. } if representation == ValueType::Int => {}
                ValidationKind::Items { .. } if representation == ValueType::StringList => {}
                ValidationKind::Pattern { .. } if representation == ValueType::String => {}
                ValidationKind::SameAs { other } => {
                    let other_ty = *field_types.get(other).ok_or_else(|| {
                        CompileError::Syntax(format!(
                            "same validation references unknown field `{other}`"
                        ))
                    })?;
                    if ty != other_ty {
                        return Err(CompileError::Syntax(format!(
                            "same validation requires matching field types `{}` and `{other}`",
                            v.field
                        )));
                    }
                    if matches!(ty, ValueType::Upload | ValueType::Image) {
                        return Err(CompileError::Syntax(format!(
                            "same validation does not support Upload/Image field `{}`",
                            v.field
                        )));
                    }
                }
                _ => {
                    return Err(CompileError::Syntax(format!(
                        "validation kind does not match field `{}` type",
                        v.field
                    )));
                }
            }
        }
        apply_default_external_input_bounds(
            path_fields
                .iter()
                .chain(query_fields.iter())
                .chain(form_fields.iter())
                .chain(json_fields.iter())
                .chain(multipart_fields.iter())
                .cloned(),
            &mut validations,
        );

        if p.routes
            .iter()
            .any(|r| r.method == method && r.path == path)
        {
            return Err(CompileError::DuplicateRoute(format!(
                "{} {}",
                t[i + 2],
                path
            )));
        }
        p.routes.push(Route {
            name,
            method,
            path,
            segments,
            query_fields,
            form_fields,
            form_schema,
            json_fields,
            multipart_fields,
            multipart_schema,
            upload,
            validations,
            tenant_field,
            auth,
            rate_policy,
            budget_profile,
            idempotent,
            public_cache,
            invalidate_caches,
            handler,
        });
    }
    Ok(())
}

pub(super) fn parse_typed_binding(
    route: &str,
    raw: &str,
    namespace: &str,
    p: &Program,
) -> Result<FormField, CompileError> {
    let lt = raw.find('<').ok_or_else(|| {
        CompileError::Syntax(format!(
            "route `{route}` binding `{raw}` must use name<Type>"
        ))
    })?;
    if !raw.ends_with('>') {
        return Err(CompileError::Syntax("malformed typed binding".into()));
    }
    let name = &raw[..lt];
    if !is_identifier(name) {
        return Err(CompileError::Syntax(format!(
            "route `{route}` invalid binding name `{name}`"
        )));
    }
    let ty = resolve_value_type(&raw[lt + 1..raw.len() - 1], namespace, p)
        .filter(|t| *t != ValueType::Upload)
        .ok_or_else(|| CompileError::Syntax("unsupported route scalar binding type".into()))?;
    Ok(FormField {
        name: name.into(),
        ty,
    })
}
fn parse_route_segments(
    route: &str,
    path: &str,
    namespace: &str,
    p: &Program,
) -> Result<(Vec<RouteSegment>, Vec<language_core::ValidationRule>), CompileError> {
    if !path.starts_with('/') {
        return Err(CompileError::Syntax(format!(
            "route `{route}` path must start /"
        )));
    }
    if path == "/" {
        return Ok((vec![], vec![]));
    }
    let mut out = Vec::new();
    let mut validations = Vec::new();
    for raw in path.trim_start_matches('/').split('/') {
        if raw.is_empty() {
            return Err(CompileError::Syntax(format!(
                "route `{route}` contains empty path segment"
            )));
        }
        if let Some(v) = raw.strip_prefix(':') {
            let f = parse_typed_binding(route, v, namespace, p)?;
            validations.extend(domain_validation::rules_for_binding(
                v, &f.name, namespace, p,
            ));
            out.push(RouteSegment::Param {
                name: f.name,
                ty: f.ty,
            });
        } else {
            out.push(RouteSegment::Static(raw.into()));
        }
    }
    Ok((out, validations))
}
pub(super) fn validate_routes(p: &Program) -> Result<(), CompileError> {
    for r in &p.routes {
        let mut expected: Vec<(String, ValueType)> = r
            .segments
            .iter()
            .filter_map(|s| match s {
                RouteSegment::Param { name, ty } => Some((name.clone(), *ty)),
                _ => None,
            })
            .collect();
        expected.extend(r.query_fields.iter().map(|f| (f.name.clone(), f.ty)));
        expected.extend(r.form_fields.iter().map(|f| (f.name.clone(), f.ty)));
        expected.extend(r.json_fields.iter().map(|f| (f.name.clone(), f.ty)));
        expected.extend(r.multipart_fields.iter().map(|f| (f.name.clone(), f.ty)));
        if let Some(u) = &r.upload {
            if r.multipart_schema.is_none() {
                expected.push((
                    u.name.clone(),
                    if u.image {
                        ValueType::Image
                    } else {
                        ValueType::Upload
                    },
                ));
            }
        }
        let params = match r.method {
            HttpMethod::Get => p.page(&r.handler).map(|h| &h.params),
            HttpMethod::Post => p.action(&r.handler).map(|h| &h.params),
        }
        .ok_or_else(|| CompileError::UnknownHandler(r.handler.clone()))?;
        crate::permission_security::validate_handler_permission(r, p)?;
        crate::mfa_security::validate_handler_mfa(r, p)?;
        crate::budget_security::validate_route_budget(r, p)?;
        crate::webhook_security::validate_route(r, p)?;
        crate::idempotency_security::validate_route(r, p)?;
        if expected.len() != params.len() {
            return Err(CompileError::RouteParamMismatch(format!(
                "route `{}` provides {} values, handler expects {}",
                r.name,
                expected.len(),
                params.len()
            )));
        }
        for ((n, t), a) in expected.iter().zip(params) {
            if n != &a.name || *t != a.ty {
                return Err(CompileError::RouteParamMismatch(format!(
                    "route `{}` `{n}` does not match handler `{}: {:?}`",
                    r.name, a.name, a.ty
                )));
            }
        }
        if matches!(r.auth, RouteAuth::Public) {
            let has_object_auth = match r.method {
                HttpMethod::Get => p.page(&r.handler).is_some_and(|h| {
                    let PageBody::Statements(s) = &h.body;
                    crate::route_page_policy::has_object_auth(s)
                }),
                HttpMethod::Post => p.action(&r.handler).is_some_and(|h| {
                    let ActionBody::Statements(s) = &h.body;
                    action_has_object_auth(s)
                }),
            };
            if has_object_auth {
                return Err(CompileError::Syntax(format!(
                    "route `{}` uses object authorization but is public; add `auth user`, `auth mfa`, `auth role ...`, or `auth permission ...`",
                    r.name
                )));
            }
            let has_business_audit = match r.method {
                HttpMethod::Get => false,
                HttpMethod::Post => p.action(&r.handler).is_some_and(|h| {
                    let ActionBody::Statements(s) = &h.body;
                    action_has_business_audit(s)
                }),
            };
            if has_business_audit {
                return Err(CompileError::Syntax(format!(
                    "route `{}` writes business audit records but is public; add `auth user`, `auth mfa`, `auth role ...`, or `auth permission ...`",
                    r.name
                )));
            }
        }
        let canonical_params = crate::route_page_policy::canonical_slug_params(p, &r.handler);
        for param in canonical_params {
            let is_path_slug = r.segments.iter().any(|segment| {
                matches!(segment, RouteSegment::Param { name, ty } if name == param && *ty == ValueType::Slug)
            });
            if !is_path_slug {
                return Err(CompileError::Syntax(format!(
                    "route `{}` handler declares canonical slug `{param}`, but `{param}` is not a Slug path parameter",
                    r.name
                )));
            }
        }
        if r.public_cache.is_some() {
            if r.method != HttpMethod::Get {
                return Err(CompileError::Syntax(format!(
                    "route `{}` public cache is GET-only",
                    r.name
                )));
            }
            if !matches!(r.auth, RouteAuth::Public) {
                return Err(CompileError::Syntax(format!(
                    "route `{}` authenticated routes cannot use public cache",
                    r.name
                )));
            }
            let page = p
                .page(&r.handler)
                .ok_or_else(|| CompileError::UnknownHandler(r.handler.clone()))?;
            let PageBody::Statements(statements) = &page.body;
            validate_public_cache_statements(&r.name, statements, p)?;
        }
        if !r.invalidate_caches.is_empty() {
            if r.method != HttpMethod::Post {
                return Err(CompileError::Syntax(format!(
                    "route `{}` cache invalidation is POST-only",
                    r.name
                )));
            }
            for target in &r.invalidate_caches {
                let cached = p.routes.iter().find(|x| x.name == *target).ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "route `{}` invalidates unknown cache route `{target}`",
                        r.name
                    ))
                })?;
                if cached.public_cache.is_none() {
                    return Err(CompileError::Syntax(format!(
                        "route `{}` invalidates non-cached route `{target}`",
                        r.name
                    )));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod lexical_hardening_tests {
    use crate::compile_source;
    use language_core::ValidationKind;

    #[test]
    fn preserves_signed_integer_range_bounds() {
        let src = r#"
#[page] fn calculator(ctx: PageContext, a: i64, b: i64) -> Result<Html, PageError> {
    return Ok(html {<p>{{ a }} {{ b }}</p>});
}
route calculator GET "/calculator"
    query a<i64> b<i64>
    validate a range -1000000 1000000 b range -1000000 1000000
    public => calculator;
"#;
        let p = compile_source(src).expect("negative range bounds must compile");
        let route = p.routes.iter().find(|r| r.name == "calculator").unwrap();
        assert!(route.validations.iter().any(|rule| {
            rule.field == "a"
                && matches!(
                    &rule.kind,
                    ValidationKind::Range { min, max }
                        if *min == -1_000_000 && *max == 1_000_000
                )
        }));
    }

    #[test]
    fn rejects_unknown_route_characters_instead_of_dropping_them() {
        let src = r#"
#[page] fn calculator(ctx: PageContext, a: i64) -> Result<Html, PageError> {
    return Ok(html {<p>{{ a }}</p>});
}
route calculator GET "/calculator" query a<i64> validate a range @1 10 public => calculator;
"#;
        let err = compile_source(src).expect_err("unknown route punctuation must fail closed");
        let msg = err.to_string();
        assert!(msg.contains("unexpected character `@`"), "{msg}");
    }

    #[test]
    fn route_like_html_text_is_not_a_route_declaration() {
        let src = r#"
#[page] fn index(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {
        <p>route fake GET /admin =&gt; missing</p>
    });
}
route index GET "/" public => index;
"#;
        let p = compile_source(src).expect("route-like HTML text must remain HTML text");
        assert_eq!(p.routes.len(), 1);
        assert_eq!(p.routes[0].name, "index");
    }

    #[test]
    fn declaration_like_html_text_is_not_parsed_as_top_level_code() {
        let src = r#"
#[page] fn index(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {
        <p>model Ghost and #[page] fn fake are documentation text.</p>
    });
}
route index GET "/" public => index;
"#;
        let p = compile_source(src)
            .expect("declaration-like HTML text must be ignored by declaration scanners");
        assert!(p.models.is_empty());
        assert_eq!(p.pages.len(), 1);
        assert_eq!(p.pages[0].name, "index");
    }
    #[test]
    fn multiline_post_route_accepts_indented_form_continuation() {
        let src = r#"
#[page] fn contact_form(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {<p>Contact</p>});
}

#[action] fn contact_submit(ctx: ActionContext, name: String) -> Result<Json, PageError> {
    return Ok(json(name));
}

route contact_form GET "/contact" public => contact_form;
route contact_submit POST "/contact"
form name<String>
public => contact_submit;
"#;
        let p = compile_source(src).expect("multiline form route must compile");
        assert_eq!(p.routes.len(), 2);
        let post = p
            .routes
            .iter()
            .find(|route| route.name == "contact_submit")
            .expect("POST route must be present");
        assert_eq!(post.form_fields.len(), 1);
        assert_eq!(post.form_fields[0].name, "name");
    }
}
