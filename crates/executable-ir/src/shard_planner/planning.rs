use super::{ShardId, VerifiedShard, fingerprint};
use crate::{ScalarStatement, VerifiedExecutableProgram, VerifiedHandler, VerifiedPureFunction};
use std::collections::{BTreeMap, BTreeSet};

/// Coarse, fixed buckets keep rustc/linker invocation count low while ensuring an
/// unrelated handler edit invalidates only one compilation unit. Qualified handlers
/// from the same source module share affinity and therefore stay together.
pub const DEFAULT_NATIVE_SHARD_BUCKETS: u32 = 4;

pub fn plan(program: &VerifiedExecutableProgram) -> Vec<VerifiedShard> {
    let mut groups: BTreeMap<(bool, u32), BTreeMap<String, VerifiedHandler>> = BTreeMap::new();
    for (name, handler) in program.handlers() {
        let native = handler.native_scalar_body().is_some();
        let bucket = stable_bucket(module_affinity(name), DEFAULT_NATIVE_SHARD_BUCKETS);
        groups
            .entry((native, bucket))
            .or_default()
            .insert(name.clone(), handler.clone());
    }

    groups
        .into_iter()
        .map(|((native_eligible, bucket), handlers)| {
            let mut capabilities = BTreeSet::new();
            for handler in handlers.values() {
                capabilities.extend(handler.manifest().capabilities.iter().cloned());
            }
            let kind = if native_eligible { "native" } else { "vm" };
            let handler_interfaces: BTreeMap<String, String> = handlers
                .iter()
                .map(|(name, handler)| (name.clone(), fingerprint::interface_sha256(handler)))
                .collect();
            let mut handler_implementations: BTreeMap<String, String> = handlers
                .iter()
                .map(|(name, handler)| (name.clone(), fingerprint::implementation_sha256(handler)))
                .collect();
            let pure_functions = if native_eligible {
                referenced_pure_functions(program, handlers.values())
            } else {
                BTreeMap::new()
            };
            for (name, function) in &pure_functions {
                handler_implementations.insert(
                    format!("pure::{name}"),
                    fingerprint::pure_function_sha256(function),
                );
            }
            let struct_schemas = referenced_struct_schemas(program, &pure_functions);
            for (id, schema) in &struct_schemas {
                handler_implementations.insert(
                    format!("struct::{id}"),
                    fingerprint::struct_schema_sha256(schema),
                );
            }
            let interface_sha256 = fingerprint::shard_sha256(
                "interface",
                handler_interfaces
                    .iter()
                    .map(|(name, value)| (name.as_str(), value.as_str())),
            );
            let implementation_sha256 = fingerprint::shard_sha256(
                "implementation",
                handler_implementations
                    .iter()
                    .map(|(name, value)| (name.as_str(), value.as_str())),
            );
            VerifiedShard {
                id: ShardId::new(format!("{kind}-b{bucket:02}")),
                handlers,
                pure_functions,
                struct_schemas,
                capabilities,
                language_version: program.language_version().to_owned(),
                security_policy_version: program.security_policy_version().to_owned(),
                executable_ir_version: program.executable_ir_version(),
                native_eligible,
                interface_sha256,
                implementation_sha256,
            }
        })
        .collect()
}

fn referenced_pure_functions<'a>(
    program: &VerifiedExecutableProgram,
    handlers: impl Iterator<Item = &'a VerifiedHandler>,
) -> BTreeMap<String, VerifiedPureFunction> {
    fn collect(statements: &[ScalarStatement], names: &mut BTreeSet<String>) {
        for statement in statements {
            match statement {
                ScalarStatement::PureCall { function, .. } => {
                    names.insert(function.clone());
                }
                ScalarStatement::If { statements, .. }
                | ScalarStatement::While { statements, .. } => collect(statements, names),
                _ => {}
            }
        }
    }

    let mut names = BTreeSet::new();
    for handler in handlers {
        if let Some(body) = handler.native_scalar_body() {
            collect(body.statements(), &mut names);
        }
    }
    names
        .into_iter()
        .filter_map(|name| {
            program
                .pure_function(&name)
                .cloned()
                .map(|function| (name, function))
        })
        .collect()
}

fn module_affinity(name: &str) -> &str {
    name.rsplit_once("::").map_or(name, |(module, _)| module)
}

fn stable_bucket(name: &str, bucket_count: u32) -> u32 {
    debug_assert!(bucket_count > 0);
    let mut hash = 0xcbf29ce484222325u64;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash % u64::from(bucket_count)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_bucket_is_deterministic_and_bounded() {
        let first = stable_bucket(module_affinity("users::get"), DEFAULT_NATIVE_SHARD_BUCKETS);
        assert_eq!(
            first,
            stable_bucket(module_affinity("users::get"), DEFAULT_NATIVE_SHARD_BUCKETS)
        );
        assert!(first < DEFAULT_NATIVE_SHARD_BUCKETS);
    }

    #[test]
    fn bucket_assignment_does_not_depend_on_other_handlers() {
        let before = stable_bucket(module_affinity("homepage"), DEFAULT_NATIVE_SHARD_BUCKETS);
        let _unrelated = stable_bucket(
            module_affinity("new_admin_page"),
            DEFAULT_NATIVE_SHARD_BUCKETS,
        );
        let after = stable_bucket(module_affinity("homepage"), DEFAULT_NATIVE_SHARD_BUCKETS);
        assert_eq!(before, after);
    }

    #[test]
    fn handlers_in_same_module_have_same_affinity() {
        assert_eq!(
            module_affinity("users::get"),
            module_affinity("users::update")
        );
        assert_eq!(
            stable_bucket(module_affinity("users::get"), DEFAULT_NATIVE_SHARD_BUCKETS),
            stable_bucket(
                module_affinity("users::update"),
                DEFAULT_NATIVE_SHARD_BUCKETS
            ),
        );
    }
}

fn referenced_struct_schemas(
    program: &VerifiedExecutableProgram,
    pure_functions: &BTreeMap<String, VerifiedPureFunction>,
) -> BTreeMap<u16, language_core::JsonSchema> {
    let mut ids = BTreeSet::new();
    for function in pure_functions.values() {
        if let language_core::PureReturnType::Value(language_core::PureValueType::Struct(name)) =
            function.return_type()
        {
            if let Some((id, _)) = program
                .struct_schemas()
                .iter()
                .find(|(_, schema)| schema.name == name)
            {
                ids.insert(*id);
            }
        }
    }
    ids.into_iter()
        .filter_map(|id| {
            program
                .struct_schemas()
                .get(&id)
                .cloned()
                .map(|schema| (id, schema))
        })
        .collect()
}
