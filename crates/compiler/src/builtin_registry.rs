use language_core::BuiltinFunction;

pub(super) fn resolve(name: &str) -> Option<BuiltinFunction> {
    BuiltinFunction::from_source_name(name)
}

#[cfg(test)]
mod tests {
    use super::resolve;
    use language_core::BuiltinFunction;

    #[test]
    fn registry_resolves_public_builtin_names() {
        assert_eq!(resolve("regexMatch"), Some(BuiltinFunction::RegexMatch));
        assert_eq!(resolve("split"), Some(BuiltinFunction::Split));
        assert_eq!(resolve("notABuiltin"), None);
    }
}
