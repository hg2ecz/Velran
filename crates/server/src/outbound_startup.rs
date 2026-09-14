use crate::WebSecurityCliConfig;
use crate::outbound_runtime::ServerOutbound;
use crate::server_config_file::DomainRuntime;
use crate::server_errors::StartupError;
use language_core::Effect;
use observability::server_log;
use std::sync::Arc;

pub(super) fn build(
    all_domains: &[Arc<DomainRuntime>],
    web: &WebSecurityCliConfig,
) -> Result<Option<Arc<ServerOutbound>>, StartupError> {
    let mut targets = std::collections::BTreeSet::new();
    for domain in all_domains {
        for effect in domain
            .program
            .pages
            .iter()
            .flat_map(|page| page.effects.iter())
            .chain(
                domain
                    .program
                    .actions
                    .iter()
                    .flat_map(|action| action.effects.iter()),
            )
        {
            if let Effect::Network(target) = effect {
                targets.insert(target.clone());
            }
        }
    }
    if targets.is_empty() {
        return Ok(None);
    }
    let Some(path) = web.egress_policy_file.as_deref() else {
        return Err(StartupError::invalid(
            "application declares outbound integration effects but web.egress_policy_file is not configured",
        ));
    };
    let client = ServerOutbound::from_policy_file(path)
        .map_err(|err| StartupError::invalid(format!("invalid egress policy: {err}")))?;
    for target in targets {
        client.validate_target(&target).map_err(|err| {
            StartupError::invalid(format!(
                "Velran outbound target `{target}` is unavailable or ambiguous in egress policy: {err}"
            ))
        })?;
    }
    server_log("outbound egress: policy loaded");
    Ok(Some(Arc::new(client)))
}
