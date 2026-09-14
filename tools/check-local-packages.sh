#!/bin/sh
set -eu
need(){ grep -F "$1" "$2" >/dev/null || { echo "missing local-package invariant: $1 in $2" >&2; exit 1; }; }
need 'struct LocalDependency' crates/compiler/src/package_manifest.rs
need 'MAX_PACKAGE_COUNT' crates/compiler/src/package_manifest.rs
need 'MAX_PACKAGE_DEPTH' crates/compiler/src/package_manifest.rs
need 'local package dependency cycle' crates/compiler/src/package_manifest.rs
need 'resolves to multiple package roots' crates/compiler/src/package_manifest.rs
need 'git, registry, version, and build-script sources are not supported' crates/compiler/src/package_manifest.rs
need 'Visibility::Private' crates/compiler/src/source_loader.rs
need 'SEC-PKG-001' crates/compiler/src/library_surface.rs
need 'parse_in_package' crates/compiler/src/module_header.rs
need 'source_roots' crates/compiler/src/compile_api.rs
need 'textkit = { path = "../libs/textkit" }' examples/local-packages/app/velran.toml
need 'transitive_local_library_dependency_compiles_without_leaking_namespace' crates/compiler/src/tests/package_compile_tests.rs
need 'transitive_dependency_is_private_to_declaring_library' crates/compiler/src/tests/package_compile_tests.rs
need 'transitive_dependency_cycle_is_rejected' crates/compiler/src/tests/package_compile_tests.rs
need 'same_package_name_cannot_resolve_to_multiple_roots' crates/compiler/src/tests/package_compile_tests.rs
echo 'local package/library verification passed'
