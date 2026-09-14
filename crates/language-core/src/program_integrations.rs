use crate::{Integration, Program};

impl Program {
    pub fn integration(&self, name: &str) -> Option<&Integration> {
        self.integrations.iter().find(|value| value.name == name)
    }
}
