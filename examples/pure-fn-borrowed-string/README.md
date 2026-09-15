<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Borrowed string pure function

Demonstrates Rust-like `&str` parameters on verified pure functions. The generated shard uses the framework-owned bounded string value and does not expose raw pointers, filesystem, network, process, environment, thread, FFI, or unsafe authority to application code.

Borrowed-string helpers participate in the current verified pure surface: they may return the supported scalar/owned/string-list/struct/`Option<T>`/`Result<T,E>` values, call scalar/borrowed pure helpers with typed expression arguments, use Rust-like branching, and recurse within the generated pure-call depth/resource limits. See [`docs/58-verified-pure-functions.md`](../../docs/58-verified-pure-functions.md).
