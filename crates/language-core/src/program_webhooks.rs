use crate::{Program, Webhook};

impl Program {
    pub fn webhook(&self, name: &str) -> Option<&Webhook> {
        self.webhooks.iter().find(|value| value.name == name)
    }
}
