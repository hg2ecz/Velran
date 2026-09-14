use crate::outbound_runtime::ServerOutbound;
use crate::server_errors::SourceReloadError;
use language_core::{Effect, Program};

pub(super) fn validate_outbound(
    program: &Program,
    outbound: Option<&ServerOutbound>,
) -> Result<(), SourceReloadError> {
    for effect in program
        .pages
        .iter()
        .flat_map(|page| page.effects.iter())
        .chain(
            program
                .actions
                .iter()
                .flat_map(|action| action.effects.iter()),
        )
    {
        if let Effect::Network(target) = effect {
            let Some(client) = outbound else {
                return Err(SourceReloadError::OutboundUnavailable);
            };
            if client.validate_target(target).is_err() {
                return Err(SourceReloadError::OutboundTargetUnavailable(target.clone()));
            }
        }
    }
    Ok(())
}
