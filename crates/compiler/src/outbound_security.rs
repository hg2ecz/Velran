use crate::diagnostics::CompileError;
use crate::module_namespace::resolve;
use language_core::{Effect, Program};
use std::collections::BTreeSet;

pub(super) fn parse_handler_effects(
    handler: &str,
    raw: &str,
    namespace: &str,
    program: &Program,
) -> Result<Vec<Effect>, CompileError> {
    if raw.trim().is_empty() || raw.contains(" uses ") {
        return Err(syntax_error(handler));
    }
    let mut effects = BTreeSet::new();
    for source_name in raw.split(',').map(str::trim) {
        if source_name.is_empty() || source_name.split_whitespace().count() != 1 {
            return Err(syntax_error(handler));
        }
        let integration_name = resolve(namespace, source_name);
        let integration = program.integration(&integration_name).ok_or_else(|| {
            CompileError::security(
                "SEC-SSRF-003",
                format!("handler `{handler}` uses unknown integration `{source_name}`"),
                Some("declare it once with `integration Name { egress target }`; arbitrary URL/network capabilities are not allowed".into()),
            )
        })?;
        let effect = Effect::Network(integration.egress_target.clone());
        if !effects.insert(effect) {
            return Err(CompileError::Syntax(format!(
                "handler `{handler}` declares duplicate outbound integration/effect `{source_name}`"
            )));
        }
    }
    Ok(effects.into_iter().collect())
}

fn syntax_error(handler: &str) -> CompileError {
    CompileError::Syntax(format!(
        "handler `{handler}` outbound capability syntax is `uses <Integration>[, <Integration> ...]`"
    ))
}
