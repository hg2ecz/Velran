<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Borrowed string pure function

Demonstrates Rust-like `&str` parameters on verified pure functions. The generated shard uses the framework-owned bounded string value and does not expose raw pointers, filesystem, network, process, environment, thread, FFI, or unsafe authority to application code.

The helpers return scalar values only in this iteration. Owned `String`, collections, structs, `Option<T>`, and `Result<T,E>` returns remain deliberately unsupported until their ownership ABI is verified.
