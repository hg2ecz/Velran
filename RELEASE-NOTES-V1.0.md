<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Velran V1 release notes

**Canonical project name:** Velran — a security-first web application language and server with Rust syntax.

Velran V1 is the Rust-first, native-only web application language/runtime/server baseline represented by this release tree. English documentation is canonical; Hungarian developer/operator documentation is maintained under `docs/hu/`.

## Language and execution model

- Rust-like surface for ordinary computation: `fn`, `let`, `let mut`, direct/compound assignment, Rust primitive/container spellings, method syntax and `vec![...]`.
- Velran-specific syntax remains where it carries web/security semantics: routes, typed request schemas, authorization policies, HTML/template boundaries, resource contracts and named capabilities.
- Application execution is verified IR -> generated safe Rust -> `rustc` -> immutable `cdylib` -> atomic activation. There is no VM/interpreter fallback. Unsupported native lowering or host-ABI coverage fails closed and leaves the previous valid generation active.

## Security and operations

- Explicit route access policy (`public`, authenticated/permission/MFA/critical policies).
- Typed and bounded request inputs, domain/nominal values, resource budgets and security-event/redaction foundations.
- Transactional source reload, content-addressed native artifacts, supply-chain capability/provenance verification and reproducible `--locked` dependency resolution.
- Production configuration, migration, backup/restore/rollback and release evidence are part of the supported operating model.

## Verification

The release tree is intended to pass:

```bash
cargo build --release --locked
./verify.sh
```

For the full current security posture, see [`SECURITY-STATUS.md`](SECURITY-STATUS.md). For native coverage, see [`tests/NATIVE_STATUS.md`](tests/NATIVE_STATUS.md).

## Iteration 6: deterministic module/import surface

- Child `mod` declarations now resolve relative to the declaring module; `crate::`, `self::`, and `super::` are explicit anchors.
- `pub mod` provides a real nested library visibility boundary without changing route/capability authority.
- `use path as alias;` provides explicit namespace-prefix aliases. A narrow package-root `pub use path as alias;` form is supported only for already-public module namespaces; wildcard/bare-symbol imports, direct item re-exports, private-module re-exports, and private transitive-dependency re-exports remain fail-closed.
- Module/import parsing is centralized in `compiler::module_header`; source loading remains responsible for confined filesystem traversal.
- Added the canonical `examples/module-imports/` release-gated example and static regression guard.
- Removed a test-only `Result<_, String>` native-build API and split two oversized executable-IR/codegen responsibilities instead of raising architecture budgets.

## Countdown iterations 5B → 1

The final countdown rounds consolidated the Rust-like package/object surface, narrowed safe re-export semantics, optimized verified native hot paths, reduced compiler/native pipeline overhead, and finished release-hardening/clean-code extraction without widening web authority. Notable current contracts include associated functions such as `Type::new(...)`, `Self` in the supported inherent-impl subset, cross-package public struct/method APIs, package-root-only explicit module re-export, chunked HTML escaping, borrowed read-only string/map fast paths, and stricter panic avoidance at security-critical boundaries.

### Verification status at handoff

This source tree has now reached the **Verified Development Milestone** on a real Rust toolchain: `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving completed successfully on the current tree. The repository-level verification state is therefore finalized for this milestone.

Production release acceptance remains a separate decision. Complete the target-environment deployment, recovery, secrets/TLS/reverse-proxy, operator, and any additional environment-specific performance/security evidence required by [`RELEASE-CHECKLIST.md`](RELEASE-CHECKLIST.md).
