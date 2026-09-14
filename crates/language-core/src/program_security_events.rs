use crate::{Program, SecurityEvent};

impl Program {
    pub fn security_event(&self, name: &str) -> Option<&SecurityEvent> {
        self.security_events.iter().find(|value| value.name == name)
    }
}
