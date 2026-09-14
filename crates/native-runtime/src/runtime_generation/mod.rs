#![forbid(unsafe_code)]

mod active;
mod binding;
mod errors;
mod generation;
mod pin;

pub use active::ActiveGeneration;
pub use binding::HandlerBinding;
pub use errors::{ActivationError, DispatchError};
pub use generation::ModuleGeneration;
pub use pin::GenerationPin;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn old_request_pin_survives_activation() {
        let active = ActiveGeneration::new(ModuleGeneration::new(1, BTreeMap::new()));
        let old = active.pin().expect("pin generation 1");
        active
            .activate(ModuleGeneration::new(2, BTreeMap::new()))
            .expect("activate generation 2");
        let new = active.pin().expect("pin generation 2");
        assert_eq!(old.number(), 1);
        assert_eq!(new.number(), 2);
        assert_eq!(active.retired_count().unwrap(), 1);
        assert_eq!(active.reap_retired().unwrap(), 0);
        drop(old);
        assert_eq!(active.reap_retired().unwrap(), 1);
    }

    #[test]
    fn activation_is_monotonic() {
        let active = ActiveGeneration::new(ModuleGeneration::new(5, BTreeMap::new()));
        assert!(matches!(
            active.activate(ModuleGeneration::new(5, BTreeMap::new())),
            Err(ActivationError::GenerationNotMonotonic { .. })
        ));
    }
}
