<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Library visibility

This example demonstrates the Rust-like Velran library boundary introduced for larger projects:

- library items are private by default;
- `pub struct`, `pub fn`, public fields, and `pub` inherent methods form the explicit cross-module API;
- `impl` blocks themselves are never `pub`;
- framework callables still use route/capability policy rather than Rust visibility.

`Summary::hidden` and `Summary::hidden_value()` are intentionally private implementation details.

Check with:

```bash
cargo run --locked -q -p velran-cli -- check examples/library-visibility/main.vrn
```
