<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Inherent methods

This example exercises the Rust-first inherent-method surface without creating a raw-Rust escape hatch.
`impl Summary` is compiler syntax, but the `&self` method body is lowered through the same verified pure-compute path as an ordinary pure `fn`; it is not pasted into generated Rust.

Current iteration-8 boundary:

- inherent `impl Type { ... }` only;
- immutable `&self` receiver;
- no owned `self` or `&mut self`;
- no trait/generic impls yet;
- method bodies remain subject to Velran security, fuel and allocation verification;
- the generated safe Rust is still typechecked and optimized by the real `rustc`.
