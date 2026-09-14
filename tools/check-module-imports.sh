#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

need() { grep -F "$1" "$2" >/dev/null || { echo "missing module/import invariant: $1 in $2" >&2; exit 1; }; }
need 'pub(crate) struct ModuleHeader' crates/compiler/src/module_header.rs
need 'use path as alias;' crates/compiler/src/module_header.rs
need 'pub use ' crates/compiler/src/module_header.rs
need 'wildcard imports/re-exports are not supported' crates/compiler/src/module_header.rs
need '`pub use` is restricted to the package root' crates/compiler/src/module_header.rs
need 'public re-export target' crates/compiler/src/source_loader/reexports.rs
need 'crosses private module' crates/compiler/src/source_loader/reexports.rs
need 'module namespace, not an item' crates/compiler/src/source_loader/reexports.rs
need 'module_path_is_accessible_from' crates/language-core/src/program_visibility.rs
need 'pub mod' crates/compiler/src/visibility.rs
need 'examples/module-imports/main.vrn' verify.sh
need 'use catalog::math as math;' examples/module-imports/pages.vrn
need 'pub mod math;' examples/module-imports/catalog.vrn

echo 'deterministic module/import verification passed'
