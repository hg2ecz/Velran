use crate::model::{
    Capability, EXECUTABLE_IR_VERSION, HandlerManifest, VerifiedExecutableProgram, VerifiedHandler,
    VerifiedHandlerBody,
};
use crate::pure_model::VerifiedPureFunction;
use language_core::{Effect, EffectClass, Program, QueryCapability, RouteAuth};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    DuplicateHandler(String),
    MissingHandler(String),
    UnknownQuery {
        handler: String,
        query: String,
    },
    UnknownPureFunction {
        handler: String,
        function: String,
    },
    RouteCapabilityMismatch {
        route: String,
        handler: String,
    },
    EffectCapabilityMismatch {
        handler: String,
        capability: Capability,
    },
    InvalidNativeScalarContract {
        handler: String,
    },
    InvalidNativeInputContract {
        handler: String,
        route: String,
    },
}

pub fn verify(program: &Program) -> Result<VerifiedExecutableProgram, VerifyError> {
    let query_caps: BTreeMap<&str, QueryCapability> = program
        .queries
        .iter()
        .map(|query| (query.name.as_str(), query.capability))
        .collect();
    let (mut route_names, mut route_caps) = collect_route_contracts(program);
    let mut pure_functions = BTreeMap::new();
    for function in &program.pure_functions {
        let scalar =
            crate::native_scalar::lower_pure_function(function, program).map_err(|_| {
                VerifyError::InvalidNativeScalarContract {
                    handler: function.name.clone(),
                }
            })?;
        let verified = VerifiedPureFunction {
            name: function.name.clone(),
            inline: function.inline,
            params: function
                .params
                .iter()
                .map(|p| (p.name.clone(), p.ty))
                .collect(),
            return_type: function.return_type.clone(),
            body: scalar,
        };
        if pure_functions
            .insert(function.name.clone(), verified)
            .is_some()
        {
            return Err(VerifyError::DuplicateHandler(function.name.clone()));
        }
    }
    let mut handlers = BTreeMap::new();
    for page in &program.pages {
        let native_inputs_allowed =
            crate::native_input::contract(program, &page.name, &page.params)?;
        let body = VerifiedHandlerBody::Page(page.body.clone());
        let handler = build_handler(
            &page.name,
            &page.effects,
            &page.security.required_permission,
            page.security.mfa_required,
            &page.security.critical_operation,
            route_names.remove(&page.name).unwrap_or_default(),
            route_caps.remove(&page.name).unwrap_or_default(),
            body,
            &page.params,
            program,
            native_inputs_allowed,
            &query_caps,
        )?;
        insert_unique(&mut handlers, &page.name, handler)?;
    }
    for action in &program.actions {
        let native_inputs_allowed =
            crate::native_input::contract(program, &action.name, &action.params)?;
        let body = VerifiedHandlerBody::Action(action.body.clone());
        let handler = build_handler(
            &action.name,
            &action.effects,
            &action.security.required_permission,
            action.security.mfa_required,
            &action.security.critical_operation,
            route_names.remove(&action.name).unwrap_or_default(),
            route_caps.remove(&action.name).unwrap_or_default(),
            body,
            &action.params,
            program,
            native_inputs_allowed,
            &query_caps,
        )?;
        insert_unique(&mut handlers, &action.name, handler)?;
    }
    if let Some((handler, routes)) = route_names.into_iter().next() {
        let route = routes.into_iter().next().unwrap_or_else(|| handler.clone());
        return Err(VerifyError::MissingHandler(format!(
            "{handler} (route {route})"
        )));
    }

    let struct_schemas = program
        .json_schemas
        .iter()
        .enumerate()
        .filter_map(|(index, schema)| u16::try_from(index).ok().map(|id| (id, schema.clone())))
        .collect();
    Ok(VerifiedExecutableProgram {
        pure_functions,
        handlers,
        struct_schemas,
        language_version: language_core::LANGUAGE_VERSION,
        security_policy_version: language_core::SECURITY_POLICY_VERSION,
        executable_ir_version: EXECUTABLE_IR_VERSION,
    })
}
type RouteContracts = (
    BTreeMap<String, BTreeSet<String>>,
    BTreeMap<String, BTreeSet<Capability>>,
);

fn collect_route_contracts(program: &Program) -> RouteContracts {
    let mut names: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut caps: BTreeMap<String, BTreeSet<Capability>> = BTreeMap::new();
    for route in &program.routes {
        names
            .entry(route.handler.clone())
            .or_default()
            .insert(route.name.clone());
        let handler_caps = caps.entry(route.handler.clone()).or_default();
        match &route.auth {
            RouteAuth::Mfa | RouteAuth::PermissionMfa { .. } => {
                handler_caps.insert(Capability::Mfa);
            }
            _ => {}
        }
        match &route.auth {
            RouteAuth::Permission { name, .. } | RouteAuth::PermissionMfa { name, .. } => {
                handler_caps.insert(Capability::Permission(name.clone()));
            }
            _ => {}
        }
    }
    (names, caps)
}
#[allow(clippy::too_many_arguments)]
fn build_handler(
    name: &str,
    effects: &[Effect],
    required_permission: &Option<String>,
    mfa_required: bool,
    critical_operation: &Option<String>,
    route_names: BTreeSet<String>,
    route_caps: BTreeSet<Capability>,
    body: VerifiedHandlerBody,
    params: &[language_core::FunctionParam],
    program: &Program,
    native_inputs_allowed: bool,
    query_caps: &BTreeMap<&str, QueryCapability>,
) -> Result<VerifiedHandler, VerifyError> {
    let mut capabilities: BTreeSet<Capability> =
        effects.iter().map(capability_from_effect).collect();
    if let Some(permission) = required_permission {
        capabilities.insert(Capability::Permission(permission.clone()));
    }
    if mfa_required {
        capabilities.insert(Capability::Mfa);
    }
    if let Some(operation) = critical_operation {
        capabilities.insert(Capability::CriticalOperation(operation.clone()));
    }
    if !route_caps.is_subset(&capabilities) {
        return Err(VerifyError::RouteCapabilityMismatch {
            route: route_names.iter().next().cloned().unwrap_or_default(),
            handler: name.to_owned(),
        });
    }
    crate::verifier_statements::verify_body(name, &body, &capabilities, query_caps, program)?;
    let native_scalar_body = if native_inputs_allowed {
        match &body {
            VerifiedHandlerBody::Page(page) => {
                crate::native_scalar::lower_page(page, params, program)
            }
            VerifiedHandlerBody::Action(action) => {
                crate::native_scalar::lower_action(action, params, program)
            }
        }
        .map_err(|_| VerifyError::InvalidNativeScalarContract {
            handler: name.to_owned(),
        })?
    } else {
        None
    };
    let mut effect_classes: BTreeSet<EffectClass> = effects.iter().map(Effect::class).collect();
    if effect_classes.is_empty() {
        effect_classes.insert(EffectClass::Pure);
    }
    Ok(VerifiedHandler {
        manifest: HandlerManifest {
            name: name.to_owned(),
            effects: effect_classes,
            capabilities,
            route_names,
        },
        body,
        native_scalar_body,
    })
}

fn insert_unique(
    handlers: &mut BTreeMap<String, VerifiedHandler>,
    name: &str,
    handler: VerifiedHandler,
) -> Result<(), VerifyError> {
    if handlers.insert(name.to_owned(), handler).is_some() {
        Err(VerifyError::DuplicateHandler(name.to_owned()))
    } else {
        Ok(())
    }
}

fn capability_from_effect(effect: &Effect) -> Capability {
    match effect {
        Effect::DbRead => Capability::DbRead,
        Effect::DbWrite => Capability::DbWrite,
        Effect::SecurityAudit => Capability::SecurityAudit,
        Effect::Network(target) => Capability::Network(target.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use language_core::{HandlerSecurityContract, PageBody, PageFunction, QueryCall, Statement};

    #[test]
    fn rejects_query_missing_from_verified_program_contract() {
        let mut program = Program::default();
        program.pages.push(PageFunction {
            name: "home".into(),
            params: vec![],
            needs_db: true,
            effects: vec![Effect::DbRead],
            security: HandlerSecurityContract::default(),
            body: PageBody::Statements(vec![Statement::LetQuery {
                name: "row".into(),
                call: QueryCall {
                    query: "missing".into(),
                    args: vec![],
                },
            }]),
        });

        assert_eq!(
            verify(&program),
            Err(VerifyError::UnknownQuery {
                handler: "home".into(),
                query: "missing".into()
            })
        );
    }

    #[test]
    fn rejects_database_use_without_declared_effect() {
        let mut program = Program::default();
        program.queries.push(language_core::QueryFunction {
            name: "load".into(),
            capability: QueryCapability::Db,
            params: vec![],
            return_type: language_core::QueryReturn::Void,
            mutation_target: None,
            tenant_scope: None,
            credential_lifecycle: None,
            sql: "SELECT 1".into(),
        });
        program.pages.push(PageFunction {
            name: "home".into(),
            params: vec![],
            needs_db: false,
            effects: vec![],
            security: HandlerSecurityContract::default(),
            body: PageBody::Statements(vec![Statement::LetQuery {
                name: "row".into(),
                call: QueryCall {
                    query: "load".into(),
                    args: vec![],
                },
            }]),
        });

        assert!(matches!(
            verify(&program),
            Err(VerifyError::EffectCapabilityMismatch {
                capability: Capability::DbRead,
                ..
            })
        ));
    }
}
