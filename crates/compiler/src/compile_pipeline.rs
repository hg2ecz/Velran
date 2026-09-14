use crate::diagnostics::CompileError;
use crate::{
    critical_declarations, critical_security, domain_objects, domain_types, handler_parser,
    inherent_impl, integration_declarations, permission_declarations, production_declarations,
    pure_functions, query_parser, routes, schema_declarations, security_event_declarations,
    source_loader, template_declarations, webhook_declarations,
};
use language_core::Program;

pub(super) fn compile_units(units: &[source_loader::SourceUnit]) -> Result<Program, CompileError> {
    let units = domain_objects::prepare_domain_units(units)?;
    let mut p = Program::default();
    // Every discovered namespace has a deterministic private-by-default module boundary.
    // Explicit `pub mod` declarations below may widen that boundary, never web authority.
    for u in &units {
        if !u.module_path.is_empty() {
            let mut owner = u.module_path.clone();
            owner.pop();
            p.set_module_visibility(
                u.module_path.join("::"),
                owner.join("::"),
                language_core::Visibility::Private,
            );
        }
    }
    for u in &units {
        let owner = u.namespace().to_owned();
        for module in &u.module_declarations {
            p.set_module_visibility(module.path.join("::"), owner.clone(), module.visibility);
        }
    }
    for u in &units {
        domain_types::parse_domain_types(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        schema_declarations::parse_enums(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        schema_declarations::parse_models(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        schema_declarations::parse_json_structs(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    // Discover inherent method signatures after structs exist, before any bodies are lowered.
    // This gives method calls a stable symbol table without admitting raw impl bodies to rustc.
    for u in &units {
        inherent_impl::register_inherent_impls(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        permission_declarations::parse_permissions(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        security_event_declarations::parse_security_events(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        critical_declarations::parse_critical_operations(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        webhook_declarations::parse_webhooks(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        integration_declarations::parse_integrations(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        production_declarations::parse_production_policy(&u.source, &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    // Pure functions use a two-phase pass: register every signature first, then lower
    // bodies. This makes cross-module/package calls deterministic and independent of
    // source discovery order while preserving visibility checks at call sites.
    for u in &units {
        pure_functions::predeclare_pure_functions(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        pure_functions::lower_pure_function_bodies(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        inherent_impl::lower_inherent_impls(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        query_parser::parse_queries(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        schema_declarations::parse_form_schemas(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    // Route signatures are parsed before page bodies so typed @href/@action helpers
    // can resolve route names and parameter types across modules while HTML is compiled.
    for u in &units {
        routes::parse_routes(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        template_declarations::parse_template_functions(&u.source, u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    template_declarations::validate_template_cycles(&p)?;
    for u in &units {
        let source_name = u.path.to_string_lossy();
        handler_parser::parse_pages(&u.source, source_name.as_ref(), u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    for u in &units {
        let source_name = u.path.to_string_lossy();
        handler_parser::parse_actions(&u.source, source_name.as_ref(), u.namespace(), &mut p)
            .map_err(|e| source_loader::source_error(u, e))?;
    }
    critical_security::validate_program(&p)?;
    routes::validate_routes(&p)?;
    if p.routes.is_empty() {
        return Err(CompileError::Syntax("no routes declared".into()));
    }
    Ok(p)
}
