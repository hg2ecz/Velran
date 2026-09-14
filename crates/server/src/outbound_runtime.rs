use integrations::{EgressPolicy, OutboundHttpsClient};
use runtime::{OutboundFuture, OutboundOutcome, OutboundRuntime};
use std::path::Path;

pub(super) struct ServerOutbound {
    client: OutboundHttpsClient,
}

impl ServerOutbound {
    pub(super) fn from_policy_file(path: &Path) -> Result<Self, integrations::IntegrationError> {
        Ok(Self {
            client: OutboundHttpsClient::new(EgressPolicy::from_toml_file(path)?),
        })
    }

    pub(super) fn validate_target(
        &self,
        target: &str,
    ) -> Result<(), integrations::IntegrationError> {
        self.client.validate_velran_target(target)
    }
}

impl OutboundRuntime for ServerOutbound {
    fn get_status<'a>(&'a self, target: &'a str, path: &'a str) -> OutboundFuture<'a> {
        Box::pin(async move {
            self.client
                .get_status_with_usage(target, path)
                .await
                .map(|v| OutboundOutcome {
                    status: v.status,
                    transferred_bytes: v.transferred_bytes,
                })
                .map_err(|_| ())
        })
    }

    fn post_json_status<'a>(
        &'a self,
        target: &'a str,
        path: &'a str,
        body: &'a [u8],
    ) -> OutboundFuture<'a> {
        Box::pin(async move {
            self.client
                .post_json_status_with_usage(target, path, body)
                .await
                .map(|v| OutboundOutcome {
                    status: v.status,
                    transferred_bytes: v.transferred_bytes,
                })
                .map_err(|_| ())
        })
    }
}
