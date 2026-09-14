<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed backend, reload, and HTTP boundaries

R12 continues the clean-code error-boundary cleanup without changing request semantics.

## Backend/runtime construction

`backend_support.rs` now returns `BackendSupportError` from listener binding, domain-runtime preparation, and hosting-runtime construction. The error keeps compiler, resource-profile, static-prefix, storage, and I/O failures distinguishable instead of erasing them behind `Box<dyn Error>`.

Configuration conflicts such as reserved media/health routes, missing upload storage, or insufficient AppFs permissions are explicit variants.

## Source reload

`source_reload.rs` now returns `SourceReloadError` for candidate validation, candidate construction, runtime commit, and cache invalidation. The type distinguishes backend construction, rate-policy validation, cache invalidation, poisoned hosting locks, TTL policy violations, missing cache/database/authentication capabilities, and therefore keeps reload rejection reasons machine-testable.

## Dispatch is infallible

`connection_dispatch::{dispatch_upload, dispatch_buffered}` never propagated an operational error: all request failures were already converted into HTTP responses. Their old `Result<DispatchOutcome, Box<dyn Error>>` signatures therefore described behavior that did not exist. They now return `DispatchOutcome` directly.

This is an important clean-code rule: do not expose a fallible API when failure is already represented in the domain result.

## Response writing

`write_response_with_timeout` only performs asynchronous writes and timeout mapping. It now returns `std::io::Result<()>` directly. Timeout is represented as `io::ErrorKind::TimedOut`.

## Boundary rule

Leaf modules should preserve the narrowest meaningful error type. `Box<dyn Error>` remains appropriate only at an intentional application aggregation boundary, such as `startup::run`, where independent subsystems genuinely converge.
