use crate::{ModuleVisibility, Program, SymbolVisibility, Visibility};

impl Program {
    pub fn set_visibility(
        &mut self,
        symbol: impl Into<String>,
        owner_namespace: impl Into<String>,
        visibility: Visibility,
    ) {
        let symbol = symbol.into();
        let owner_namespace = owner_namespace.into();
        if let Some(existing) = self
            .visibilities
            .iter_mut()
            .find(|entry| entry.symbol == symbol)
        {
            existing.owner_namespace = owner_namespace;
            existing.visibility = visibility;
            return;
        }
        self.visibilities.push(SymbolVisibility {
            symbol,
            owner_namespace,
            visibility,
        });
    }

    pub fn set_module_visibility(
        &mut self,
        module: impl Into<String>,
        owner_namespace: impl Into<String>,
        visibility: Visibility,
    ) {
        let module = module.into();
        let owner_namespace = owner_namespace.into();
        if let Some(existing) = self
            .module_visibilities
            .iter_mut()
            .find(|entry| entry.module == module)
        {
            existing.owner_namespace = owner_namespace;
            existing.visibility = visibility;
            return;
        }
        self.module_visibilities.push(ModuleVisibility {
            module,
            owner_namespace,
            visibility,
        });
    }

    pub fn module_is_accessible_from(&self, module: &str, namespace: &str) -> bool {
        if module.is_empty() {
            return true;
        }
        let Some(entry) = self
            .module_visibilities
            .iter()
            .find(|entry| entry.module == module)
        else {
            return true;
        };
        entry.visibility == Visibility::Public
            || entry.owner_namespace.is_empty()
            || namespace == entry.owner_namespace
            || namespace.starts_with(&format!("{}::", entry.owner_namespace))
    }

    pub fn module_path_is_accessible_from(&self, module: &str, namespace: &str) -> bool {
        let parts: Vec<&str> = module.split("::").filter(|part| !part.is_empty()).collect();
        (1..=parts.len())
            .all(|len| self.module_is_accessible_from(&parts[..len].join("::"), namespace))
    }

    pub fn visibility(&self, symbol: &str) -> Visibility {
        self.visibilities
            .iter()
            .find(|entry| entry.symbol == symbol)
            .map(|entry| entry.visibility)
            .unwrap_or(Visibility::Private)
    }

    pub fn symbol_is_accessible_from(&self, symbol: &str, namespace: &str) -> bool {
        let Some(entry) = self
            .visibilities
            .iter()
            .find(|entry| entry.symbol == symbol)
        else {
            return false;
        };
        if entry.visibility == Visibility::Public {
            return true;
        }
        let owner = entry.owner_namespace.as_str();
        owner.is_empty() || namespace == owner || namespace.starts_with(&format!("{owner}::"))
    }
}
