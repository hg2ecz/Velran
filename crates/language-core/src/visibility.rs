#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolVisibility {
    pub symbol: String,
    pub owner_namespace: String,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleVisibility {
    pub module: String,
    pub owner_namespace: String,
    pub visibility: Visibility,
}

#[cfg(test)]
mod tests {
    use crate::{Program, Visibility};

    #[test]
    fn private_symbol_is_visible_to_its_module_and_descendants_only() {
        let mut program = Program::default();
        program.set_visibility("catalog::Hidden", "catalog", Visibility::Private);
        assert!(program.symbol_is_accessible_from("catalog::Hidden", "catalog"));
        assert!(program.symbol_is_accessible_from("catalog::Hidden", "catalog::nested"));
        assert!(!program.symbol_is_accessible_from("catalog::Hidden", "other"));
    }

    #[test]
    fn member_uses_declaring_module_not_type_name_as_privacy_owner() {
        let mut program = Program::default();
        program.set_visibility("catalog::Summary::hidden", "catalog", Visibility::Private);
        assert!(program.symbol_is_accessible_from("catalog::Summary::hidden", "catalog::nested"));
        assert!(!program.symbol_is_accessible_from("catalog::Summary::hidden", "other"));
    }

    #[test]
    fn public_symbol_is_visible_cross_module() {
        let mut program = Program::default();
        program.set_visibility("catalog::Public", "catalog", Visibility::Public);
        assert!(program.symbol_is_accessible_from("catalog::Public", "other"));
    }
}
