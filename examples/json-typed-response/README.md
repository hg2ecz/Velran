<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed JSON response

Demonstrates `Json<T>` request binding and `Result<Json<T>, PageError>` response typing.
The generated application shard emits a bounded typed binary response frame; JSON serialization and escaping remain framework-owned.
