<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Modules, namespaces, and imports

Velran keeps module structure explicit and deterministic while leaving Rust typing and native code generation to `rustc`.

## File mapping

```text
main.vrn                application root
models.vrn              models
pages/article.vrn       pages::article
admin/users/edit.vrn    admin::users::edit
```

A module path still maps to exactly one regular non-symlink `.vrn` file. Filesystem traversal is rejected and the compiler never follows symlinks outside the application root.

## `mod` and `pub mod`

Module declarations are top-level source-graph declarations. They must appear before ordinary declarations in the source unit. Directories are never scanned automatically: every source unit that joins the module graph must be declared explicitly. This keeps loading deterministic and prevents filesystem discovery from becoming an implicit capability.

A plain child declaration is relative to the declaring module:

```velran
// pages.vrn
mod article;           // pages::article -> pages/article.vrn
```

Explicit anchors are available:

```velran
mod crate::shared;     // application root
mod self::article;     // current module
mod super::shared;     // parent module
```

`pub mod` exposes a nested module across sibling namespaces:

```velran
// catalog.vrn
pub mod math;
```

Visibility is layered. Access to an ordinary library item requires both an accessible module path and an accessible item. `pub mod` does not make private functions public, and item-level `pub` does not bypass a private nested-module boundary.

Web authority is separate: route/page/action access is still controlled by Velran route and capability policy, never by `pub mod`.

## `use` aliases

Iteration 6 intentionally supports a narrow import form:

```velran
use catalog::math as math;

let value = math::answer();
```

This aliases an explicit namespace prefix only. Velran does not implement a second Rust name/type checker, wildcard imports, implicit prelude injection, or bare-symbol shadowing rules. The alias is expanded to the canonical namespace before ordinary semantic lowering, and `rustc` remains authoritative for generated Rust typing.

Package/library ownership now enables a deliberately narrow re-export form: `pub use path as alias;` is accepted only at the package root, and only when `path` names an already-public module namespace in the same package. Wildcards, direct item re-exports, private modules, and private transitive dependencies remain fail-closed. Re-export does not grant route, action, query, integration, secret, filesystem, network, or other framework authority.

## Header order

Module/import declarations must appear before ordinary declarations in a source unit. Allowed header forms are:

```velran
mod child;
pub mod public_child;
use crate::shared as shared;
```

Duplicate module declarations and duplicate aliases are compile errors. Module cycles are detected with an explicit dependency chain in the diagnostic.

## Clean-code guidance

Prefer small cohesive modules, explicit public APIs, and short aliases only when they improve readability. A module should own one reason to change. Do not use imports to hide architectural dependencies; the canonical namespace should remain obvious from the alias and directory layout.

See `examples/module-imports/` for the canonical multi-module example.

## Local packages

An application may declare explicit local libraries in `velran.toml`. Only relative path dependencies are accepted; registry, git, version resolution and build scripts are never executed. Package paths are canonicalized and must stay inside the application workspace while remaining outside the application package itself. Package manifests and entrypoints must be regular non-symlink files/directories at their boundary.

The dependency graph is resolved recursively with explicit resource limits. Cycles are rejected with the package chain, package depth and graph size are bounded, and one package name may not resolve to multiple canonical roots. These checks happen before source lowering so ambiguous package identity fails closed.

A direct application dependency becomes a public root namespace:

```velran
let value = textkit::answer();
```

A library dependency is mounted below the declaring library and is private to that package boundary. For example, if `textkit` depends on `util`, `textkit` may write:

```velran
use util as util;

pub fn answer() -> i64 {
    return util::answer();
}
```

The canonical namespace is `textkit::util`, but the application cannot call `textkit::util::answer()` merely because `textkit` depends on `util`. A library may expose an already-public module namespace only through the narrow package-root `pub use path as alias;` contract; private transitive dependencies remain non-re-exportable. This prevents transitive dependencies from accidentally becoming application API.

Inside every package, `crate::` resolves to that package's own namespace root. Library items remain private by default and cross an accessible package/module boundary only with `pub`.

Libraries remain computation/API packages: route/page/action/query/permission/integration/production authority stays in the application package. Dependency declaration never grants web authority, filesystem authority, network authority, secret access, or build-script execution.
