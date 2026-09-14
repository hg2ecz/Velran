<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed errors and explicit module dependencies

The R8 maintainability pass continues the clean-code work without changing Velran language semantics or HTTP behavior.

## Typed resource-limit errors

The server `resource_limits` module no longer exposes `Result<_, String>` from its typed `apply` API. Failures are represented by `ResourceLimitError`, separating invalid operator configuration, unsupported platform features, operating-system I/O failures, and CPU quota overflow.

This preserves the human-readable startup diagnostics while also making errors machine-classifiable and source-chain aware through `std::error::Error`.

## Isolated rate-limit implementation

Route rate limiting is now owned by `server/src/rate_limit.rs` rather than the server bootstrap root. `RateLimitError` distinguishes unknown policies, missing authenticated principals, clock failures, backend failures, poisoned locks, and in-memory capacity exhaustion.

The server's public HTTP behavior remains fail-closed: limiter failures still produce the existing service-unavailable response instead of exposing internal details.

## Explicit compiler dependencies

Small compiler collection modules now import only the parser and type APIs that they actually use. They no longer rely on `use super::*` to obtain hidden dependencies from the crate root.

The structural guard enforces these boundaries so later root-module refactors cannot silently change the dependency surface of these modules.
