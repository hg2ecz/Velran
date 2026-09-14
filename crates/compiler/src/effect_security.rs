use crate::diagnostics::CompileError;
use language_core::{ActionStatement, Effect, Statement, TxStatement};
use std::collections::BTreeSet;

pub(super) fn merge(declared: Vec<Effect>, inferred: Vec<Effect>) -> Vec<Effect> {
    let mut effects = BTreeSet::new();
    effects.extend(declared);
    effects.extend(inferred);
    effects.into_iter().collect()
}

pub(super) fn collect_page(statements: &[Statement]) -> Vec<Effect> {
    let mut effects = BTreeSet::new();
    collect_page_into(statements, &mut effects);
    effects.into_iter().collect()
}

pub(super) fn collect_action(statements: &[ActionStatement]) -> Vec<Effect> {
    let mut effects = BTreeSet::new();
    collect_action_into(statements, &mut effects);
    effects.into_iter().collect()
}

pub(super) fn validate_db_capability(
    kind: &str,
    name: &str,
    needs_db: bool,
    effects: &[Effect],
) -> Result<(), CompileError> {
    let uses_db = effects
        .iter()
        .any(|effect| matches!(effect, Effect::DbRead | Effect::DbWrite));
    if uses_db && !needs_db {
        return Err(CompileError::security(
            "SEC-EFFECT-001",
            format!("{kind} `{name}` performs a database effect without receiving `db: Db`"),
            Some("add `db: Db` to the handler parameters; database authority is explicit and is never inferred from the identifier name `db`".into()),
        ));
    }
    Ok(())
}

pub(super) fn validate_network_capabilities(
    kind: &str,
    name: &str,
    declared: &[Effect],
    inferred: &[Effect],
) -> Result<(), CompileError> {
    for effect in inferred {
        let Effect::Network(target) = effect else {
            continue;
        };
        if !declared.iter().any(|candidate| candidate == effect) {
            return Err(CompileError::security(
                "SEC-EFFECT-002",
                format!("{kind} `{name}` performs `net.{target}` without an explicit integration capability"),
                Some("add the corresponding integration to the handler `uses ...` contract; outbound authority is explicit at the handler boundary".into()),
            ));
        }
    }
    Ok(())
}

fn collect_page_into(statements: &[Statement], effects: &mut BTreeSet<Effect>) {
    for statement in statements {
        match statement {
            Statement::LetQuery { .. } => {
                effects.insert(Effect::DbRead);
            }
            Statement::LetOutboundStatus { call, .. } => {
                effects.insert(Effect::Network(call.egress_target.clone()));
            }
            Statement::Resource { statements, .. } => collect_page_into(statements, effects),
            Statement::Match { arms, .. } => {
                for arm in arms {
                    collect_page_into(&arm.statements, effects);
                }
            }
            _ => {}
        }
    }
}

fn collect_action_into(statements: &[ActionStatement], effects: &mut BTreeSet<Effect>) {
    for statement in statements {
        match statement {
            ActionStatement::LetQuery { .. } => {
                effects.insert(Effect::DbRead);
            }
            ActionStatement::LetOutboundStatus { call, .. } => {
                effects.insert(Effect::Network(call.egress_target.clone()));
            }
            ActionStatement::Transaction { statements, .. } => {
                effects.insert(Effect::DbWrite);
                if statements
                    .iter()
                    .any(|statement| matches!(statement, TxStatement::BusinessAudit(_)))
                {
                    effects.insert(Effect::SecurityAudit);
                }
            }
            ActionStatement::Resource { statements, .. } => {
                collect_action_into(statements, effects)
            }
            ActionStatement::Match { arms, .. } => {
                for arm in arms {
                    collect_action_into(&arm.statements, effects);
                }
            }
            _ => {}
        }
    }
}
