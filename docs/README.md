<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Velran documentation

**Canonical project name:** Velran — a security-first web application language and server with Rust syntax.

English is the canonical documentation language for the Velran repository. Hungarian translations and developer/operator guides live under [`hu/`](hu/).

## Start here

- [Repository README](../README.md)
- [Current security status](../SECURITY-STATUS.md)
- [V1 release notes](../RELEASE-NOTES-V1.0.md)
- [Release checklist](../RELEASE-CHECKLIST.md)
- [Hungarian developer/operator handbook](hu/README.md)

## Core language reference

- [Modules and namespaces](21-modules-and-namespaces.md)
- [Numeric operators, f32 math and monotonic timing](44-math-and-timing.md)
- [Rust-like string operations](46-string-builtins.md)
- [String lists](47-string-lists.md)
- [String dictionaries](48-string-dictionaries.md)
- [Regular expressions](49-regular-expressions.md)
- [Statement terminators](56-statement-terminators.md)
- [Domain types and bounded collections](62-domain-types-and-bounded-collections.md)
- [Nominal domain types](63-nominal-domain-types.md)
- [Domain value refinement](64-domain-value-refinement.md)
- [Closed sum types and exhaustive match](94-sum-types-and-exhaustive-match.md)

## Security architecture

Start with [Secure-by-construction foundation](57-secure-by-construction-foundation.md), then use the focused chapters below.

### Access control and data flow

- [Authorized mutations](59-authorized-mutations.md)
- [Secure routing and egress](60-secure-routing-and-egress.md)
- [Validated input flow](61-validated-input-flow.md)
- [Explicit public projections](65-explicit-public-projections.md)
- [Secret consumer contracts](66-secret-consumer-contracts.md)
- [Function-level permissions](75-function-level-permissions.md)
- [MFA elevation proofs](76-mfa-elevation-proofs.md)
- [Critical operation contracts](77-critical-operation-contracts.md)
- [Tenant membership authority](81-tenant-membership-authority.md)
- [Compiler-enforced tenant isolation](82-compiler-enforced-tenant-isolation.md)

### Authentication, tokens and cryptography

- [Typed credentials and password primitives](67-typed-credentials-and-passwords.md)
- [Credential purpose types](68-credential-purpose-types.md)
- [Purpose-safe token primitives](69-purpose-safe-token-primitives.md)
- [Token hashes at rest](70-token-hash-at-rest.md)
- [Expiry-aware token verification](71-expiry-aware-token-verification.md)
- [Atomic token lifecycle](72-atomic-token-lifecycle.md)
- [Atomic session rotation](73-atomic-session-rotation.md)
- [Platform-owned browser sessions](74-platform-owned-browser-sessions.md)
- [Cryptographic key purpose and lifecycle](89-crypto-key-purpose-lifecycle.md)
- [Authentication abuse protection](93-authentication-abuse-protection.md)
- [Authenticated encryption and key lifecycle](96-authenticated-encryption-key-lifecycle.md)

### Integrity, I/O and deployment boundaries

- [Idempotent critical operations](83-idempotent-critical-operations.md)
- [Verified webhook integrity](81-verified-webhook-integrity.md)
- [Safe file-upload state machine](84-safe-file-upload-state-machine.md)
- [Effect/capability foundation](85-effect-capability-foundation.md)
- [Typed outbound integration calls](86-typed-outbound-integration-calls.md)
- [Production configuration policy](87-production-configuration-policy.md)
- [Typed HTTP metadata](88-typed-http-metadata.md)
- [Generated CSP and security headers](90-generated-security-headers.md)
- [Supply-chain capability and provenance](92-supply-chain-capability-provenance.md)

### Exceptional conditions, resources and monitoring

- [Public error boundaries](80-public-error-boundaries.md)
- [Route budget profiles](78-route-budget-profiles.md)
- [Bounded request collections](79-bounded-request-collections.md)
- [Closed sum types and exhaustive match](94-sum-types-and-exhaustive-match.md)
- [Transaction outcome semantics](95-transaction-outcome-semantics.md)
- [Advanced resource safety](97-advanced-resource-safety.md)
- [Security monitoring and redaction](98-security-monitoring-and-redaction.md)
- [Security architecture and current status](99-security-architecture-and-status.md)

## Operations and delivery

- [Dependency security and reproducible builds](19-dependency-security.md)
- [Optimistic locking and concurrent edits](24-optimistic-locking.md)
- [IPv6-ready outbound egress](32-ipv6-egress.md)
- [Debian package build and installation](36-debian-package.md)
- [Multi-domain hosting](37-multi-domain-hosting.md)
- [Reverse-proxy application-server mode](38-reverse-proxy-application-server.md)
- [Automatic application source reload](39-automatic-source-reload.md)
- [Maintainability and clean-code boundaries](50-maintainability-and-clean-code.md)

## Source examples as compatibility surface

User-facing Velran examples live under `examples/` and are enumerated in `tests/manifests/example-entrypoints.txt` for compatibility checking. The namespace applications live at `examples/module-namespaces/` and `examples/module-imports/`; the latter demonstrates module-relative paths, `pub mod`, and explicit namespace aliases. Compiler-rejection and security-negative fixtures live under `tests/fixtures/`, while verification manifests live under `tests/manifests/`.

## Book

The canonical English book lives under [`book/`](book/); the Hungarian edition lives under [`book/hu/`](book/hu/).

## Historical development notes

Iteration, benchmark and one-off build-fix notes are retained separately under [`history/development-notes/`](../history/development-notes/) and are non-normative. They may contain superseded syntax or architecture descriptions.

## Documentation rule

1. Write canonical documentation in English.
2. Keep Hungarian translations under `hu/` (or use an explicit `_hu` suffix where needed).
3. Prefer one canonical topic document over per-iteration verification/build-fix Markdown files.
4. Temporary build-fix notes belong in version control/review history, not in the release documentation tree.
