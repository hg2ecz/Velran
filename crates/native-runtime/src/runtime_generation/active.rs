use std::sync::{Arc, Mutex, RwLock};

use super::{ActivationError, GenerationPin, ModuleGeneration};

pub struct ActiveGeneration {
    current: RwLock<Arc<ModuleGeneration>>,
    retired: Mutex<Vec<Arc<ModuleGeneration>>>,
}

impl ActiveGeneration {
    pub fn new(initial: ModuleGeneration) -> Self {
        Self {
            current: RwLock::new(Arc::new(initial)),
            retired: Mutex::new(Vec::new()),
        }
    }
    pub fn pin(&self) -> Result<GenerationPin, ActivationError> {
        let current = self
            .current
            .read()
            .map_err(|_| ActivationError::LockPoisoned)?;
        Ok(GenerationPin(Arc::clone(&current)))
    }
    pub fn activate(&self, next: ModuleGeneration) -> Result<u64, ActivationError> {
        let mut current = self
            .current
            .write()
            .map_err(|_| ActivationError::LockPoisoned)?;
        if next.number() <= current.number() {
            return Err(ActivationError::GenerationNotMonotonic {
                current: current.number(),
                next: next.number(),
            });
        }
        let previous = Arc::clone(&current);
        let previous_number = previous.number();
        *current = Arc::new(next);
        drop(current);
        self.retired
            .lock()
            .map_err(|_| ActivationError::LockPoisoned)?
            .push(previous);
        Ok(previous_number)
    }
    pub fn reap_retired(&self) -> Result<usize, ActivationError> {
        let mut retired = self
            .retired
            .lock()
            .map_err(|_| ActivationError::LockPoisoned)?;
        let before = retired.len();
        retired.retain(|generation| Arc::strong_count(generation) > 1);
        Ok(before - retired.len())
    }
    pub fn retired_count(&self) -> Result<usize, ActivationError> {
        Ok(self
            .retired
            .lock()
            .map_err(|_| ActivationError::LockPoisoned)?
            .len())
    }
}
