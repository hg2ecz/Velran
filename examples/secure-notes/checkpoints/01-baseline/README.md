<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Secure Notes checkpoint: 01-baseline

**Chapter 1 mental model and secure baseline.**

The compiler-checked baseline: authenticated routes, principal-scoped listing, ownership checks, bounded inputs and transactional mutation.

This directory is an immutable, compiler-checkable snapshot of the Secure Notes baseline used at this learning point. The snapshot intentionally preserves the same security invariants as the canonical application; chapter-specific concepts that belong to separate framework surfaces are referenced as verified companion examples rather than being artificially merged into one application.

## Verified companions

- No additional companion is required for this checkpoint.

## Invariants that must remain true

- every route is explicitly `auth user`;
- list queries are scoped by `authPrincipal`;
- object reads/mutations use explicit ownership authorization;
- title/body request bounds remain enforced;
- mutations stay transactional;
- the database migration preserves length, state and owner/index constraints.
