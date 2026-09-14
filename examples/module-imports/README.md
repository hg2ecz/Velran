<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Module imports

Canonical example for the deterministic Velran module surface introduced in iteration 6.

- `mod child;` is relative to the declaring module.
- `pub mod child;` exposes a nested module across sibling namespaces.
- `crate::`, `self::`, and `super::` are explicit path anchors.
- `use path as alias;` imports a namespace prefix without adding a second Rust-like scope/type checker.
- `pub use` is intentionally rejected for now; public API remains explicit at the defining module.
