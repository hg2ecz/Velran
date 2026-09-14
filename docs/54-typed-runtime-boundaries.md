<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed runtime boundaries in the server

The server keeps typed errors at leaf boundaries and erases them only at the top-level startup aggregation boundary.

## Public page cache

`PublicPageCache` returns `PublicCacheError` instead of `String`. The error preserves distinct failure classes for:

- system clock failures;
- poisoned in-process cache locks;
- Redis/data backend failures;
- malformed UTF-8 generation values;
- malformed numeric generation values;
- JSON serialization/deserialization failures.

This means callers may log one user-safe message while tests and higher layers can still distinguish the original failure category.

## Clock boundary

`unix_secs()` returns `ClockError`. A clock before the Unix epoch is not flattened into text.

## Upload value construction

`build_upload_runtime_value()` returns `UploadRuntimeError`, separating storage read failures, image validation failures, and failure to construct a validated image reference.

## Listener and signal boundaries

The HTTP redirect listener and shutdown signal helper return `std::io::Error` directly. They do not need a custom enum because their leaf operations already have one precise standard-library error type.

## Aggregation rule

`startup::run()` remains the deliberate application boundary that may aggregate heterogeneous errors with `Box<dyn Error>`. Leaf modules should not introduce boxed or stringly errors merely for convenience.
