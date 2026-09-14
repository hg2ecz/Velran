use super::{SourceUnit, source_error};
use crate::diagnostics::CompileError;
use language_core::Visibility;
use std::collections::{HashMap, HashSet};

pub(super) fn apply_public_reexports(units: &mut [SourceUnit]) -> Result<(), CompileError> {
    let mut declared_visibility = HashMap::<Vec<String>, Visibility>::new();
    let mut known_modules = HashSet::<Vec<String>>::new();
    for unit in units.iter() {
        known_modules.insert(unit.module_path.clone());
        for module in &unit.module_declarations {
            declared_visibility.insert(module.path.clone(), module.visibility);
            known_modules.insert(module.path.clone());
        }
    }

    let mut mappings = Vec::<(Vec<String>, Vec<String>)>::new();
    for unit in units.iter() {
        for reexport in &unit.public_reexports {
            if unit.module_path != unit.package_root {
                return Err(source_error(
                    unit,
                    CompileError::Syntax(
                        "`pub use` is restricted to the package root in this iteration".into(),
                    ),
                ));
            }
            if !reexport.target.starts_with(&unit.package_root) {
                return Err(source_error(
                    unit,
                    CompileError::Syntax(format!(
                        "public re-export target `{}` escapes its package root",
                        reexport.target.join("::")
                    )),
                ));
            }
            if !known_modules.contains(&reexport.target) {
                return Err(source_error(
                    unit,
                    CompileError::Syntax(format!(
                        "public re-export target `{}` must name a declared module namespace, not an item",
                        reexport.target.join("::")
                    )),
                ));
            }
            for depth in unit.package_root.len() + 1..=reexport.target.len() {
                let segment = reexport.target[..depth].to_vec();
                if declared_visibility.get(&segment) != Some(&Visibility::Public) {
                    return Err(source_error(
                        unit,
                        CompileError::Syntax(format!(
                            "public re-export target `{}` crosses private module `{}`; expose the module with `pub mod` first",
                            reexport.target.join("::"),
                            segment.join("::")
                        )),
                    ));
                }
            }
            let mut exposed = unit.package_root.clone();
            exposed.push(reexport.alias.clone());
            if known_modules.contains(&exposed) && exposed != reexport.target {
                return Err(source_error(
                    unit,
                    CompileError::Syntax(format!(
                        "public re-export alias `{}` conflicts with an existing module namespace",
                        exposed.join("::")
                    )),
                ));
            }
            if mappings.iter().any(|(existing, _)| existing == &exposed) {
                return Err(source_error(
                    unit,
                    CompileError::Syntax(format!(
                        "duplicate public re-export alias `{}`",
                        exposed.join("::")
                    )),
                ));
            }
            mappings.push((exposed, reexport.target.clone()));
        }
    }

    if mappings.is_empty() {
        return Ok(());
    }
    mappings.sort_by(|(a, _), (b, _)| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    for unit in units.iter_mut() {
        for (exposed, target) in &mappings {
            let from = format!("{}::", exposed.join("::"));
            let to = format!("{}::", target.join("::"));
            unit.source = crate::module_header::replace_token_prefix(&unit.source, &from, &to);
        }
    }
    Ok(())
}
