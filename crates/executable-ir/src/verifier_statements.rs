use crate::model::{Capability, VerifiedHandlerBody};
use crate::verifier::VerifyError;
use language_core::{
    ActionBody, ActionStatement, PageBody, Program, QueryCapability, Statement, TxStatement,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn verify_body(
    handler: &str,
    body: &VerifiedHandlerBody,
    capabilities: &BTreeSet<Capability>,
    query_caps: &BTreeMap<&str, QueryCapability>,
    program: &Program,
) -> Result<(), VerifyError> {
    match body {
        VerifiedHandlerBody::Page(PageBody::Statements(statements)) => {
            for statement in statements {
                verify_page_statement(handler, statement, capabilities, query_caps, program)?;
            }
        }
        VerifiedHandlerBody::Action(ActionBody::Statements(statements)) => {
            for statement in statements {
                verify_action_statement(handler, statement, capabilities, query_caps)?;
            }
        }
    }
    Ok(())
}

fn require(
    handler: &str,
    capability: Capability,
    capabilities: &BTreeSet<Capability>,
) -> Result<(), VerifyError> {
    if capabilities.contains(&capability) {
        Ok(())
    } else {
        Err(VerifyError::EffectCapabilityMismatch {
            handler: handler.to_owned(),
            capability,
        })
    }
}

fn require_query(
    handler: &str,
    query: &str,
    capabilities: &BTreeSet<Capability>,
    query_caps: &BTreeMap<&str, QueryCapability>,
) -> Result<(), VerifyError> {
    let capability = match query_caps.get(query).copied() {
        Some(QueryCapability::Db) => Capability::DbRead,
        Some(QueryCapability::Transaction) => Capability::DbWrite,
        None => {
            return Err(VerifyError::UnknownQuery {
                handler: handler.to_owned(),
                query: query.to_owned(),
            });
        }
    };
    require(handler, capability, capabilities)
}

fn verify_page_statement(
    handler: &str,
    statement: &Statement,
    capabilities: &BTreeSet<Capability>,
    query_caps: &BTreeMap<&str, QueryCapability>,
    program: &Program,
) -> Result<(), VerifyError> {
    match statement {
        Statement::LetQuery { call, .. } => {
            require_query(handler, &call.query, capabilities, query_caps)
        }
        Statement::LetOutboundStatus { call, .. } => require(
            handler,
            Capability::Network(call.egress_target.clone()),
            capabilities,
        ),
        Statement::PureCall { function, .. } => {
            if program.pure_function(function).is_some() {
                Ok(())
            } else {
                Err(VerifyError::UnknownPureFunction {
                    handler: handler.to_owned(),
                    function: function.clone(),
                })
            }
        }
        Statement::Resource { statements, .. } => {
            for nested in statements {
                verify_page_statement(handler, nested, capabilities, query_caps, program)?;
            }
            Ok(())
        }
        Statement::Match { arms, .. } => {
            for arm in arms {
                for nested in &arm.statements {
                    verify_page_statement(handler, nested, capabilities, query_caps, program)?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn verify_action_statement(
    handler: &str,
    statement: &ActionStatement,
    capabilities: &BTreeSet<Capability>,
    query_caps: &BTreeMap<&str, QueryCapability>,
) -> Result<(), VerifyError> {
    match statement {
        ActionStatement::LetQuery { call, .. } => {
            require_query(handler, &call.query, capabilities, query_caps)
        }
        ActionStatement::Transaction { statements, .. } => {
            require(handler, Capability::DbWrite, capabilities)?;
            for statement in statements {
                match statement {
                    TxStatement::LetQuery { call, .. } | TxStatement::Query(call) => {
                        require_query(handler, &call.query, capabilities, query_caps)?;
                    }
                    TxStatement::BusinessAudit(_) => {
                        require(handler, Capability::SecurityAudit, capabilities)?
                    }
                }
            }
            Ok(())
        }
        ActionStatement::LetOutboundStatus { call, .. } => require(
            handler,
            Capability::Network(call.egress_target.clone()),
            capabilities,
        ),
        ActionStatement::Resource { statements, .. } => {
            for nested in statements {
                verify_action_statement(handler, nested, capabilities, query_caps)?;
            }
            Ok(())
        }
        ActionStatement::Match { arms, .. } => {
            for arm in arms {
                for nested in &arm.statements {
                    verify_action_statement(handler, nested, capabilities, query_caps)?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
