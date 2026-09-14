<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Secure-by-construction language foundation

Velran treats common web-security requirements as language/compiler/runtime semantics wherever they can be enforced reliably. The safe path should be the shortest path, and unsafe authority should be explicit or unrepresentable.

## Core rules

### Route access is never implicit

Every route states its access policy explicitly. Public, authenticated, permission/MFA, critical, tenant-bound and verified-webhook routes are distinct authorities; omission is a compile-time security error.

### Internal navigation is typed

Redirects use compiler-checked route calls rather than dynamic URL strings. The compiler validates target existence, GET semantics, argument count/types and constructs a `LocalUrl` at runtime. This removes the old string-based redirect surface from normal Velran code.

### External input starts untrusted and bounded

Request values carry trust state and boundary validation. Raw request strings have a safe upper bound unless a narrower domain rule is declared. Domain types centralize length/pattern/range invariants, and nominal types prevent identity-sensitive values such as different ID domains from being interchanged.

### Secrets and sensitive values keep their classification

`Secret<T>` cannot flow to generic output/audit sinks. `Sensitive<T>` disclosure requires the relevant authorization evidence. Public model output is an explicit field projection, so adding a model field cannot silently expand an API response.

### Authorization is proof-bearing

Object authorization, mutation authorization, named permissions, MFA elevation and tenant isolation produce or require compiler evidence. Validation proof never substitutes for authorization proof.

### Critical operations are contracts

Critical operations can require permission, MFA, transaction, typed audit and idempotency as one domain-level contract. Idempotent critical transactions also model explicit transaction outcomes (`Committed`, `RolledBack`, `CommitUnknown`) so unknown commit state is not silently retried.

### Ambient authority is minimized

Database and outbound-network effects are compiler-tracked. Named integrations bind Velran code to trusted egress targets without exposing arbitrary host/IP/port authority. File publication, webhook verification and browser/session authority are platform-owned boundaries.

### Resource exhaustion is a security condition

Request strings/collections, DB row sets, uploads, image dimensions, outbound responses, cumulative external I/O, instructions/allocations and whole-request execution time are bounded. Transparent untrusted decompression is intentionally absent.

### Security telemetry is structured and redacted

Typed security events and platform auto-events avoid arbitrary request payload logging. `Redacted<T>` is produced only by the trusted `redact(...)` operation, and burst detection provides an alerting foundation without exposing raw principal/source values.

## Stable diagnostics

Security diagnostics use stable `SEC-*` identifiers. OWASP-family fragments in diagnostic names are navigation aids for the relevant risk class, not a compliance claim.

## Design order

Velran prefers:

1. make the unsafe state unrepresentable;
2. otherwise prove the invariant at compile time;
3. otherwise enforce a secure platform/runtime default;
4. otherwise require an explicit capability/policy and fail closed when it is absent.

Developer ergonomics are part of the security model. Repeated security plumbing is avoided when the compiler/runtime can derive the same invariant from domain intent.

## Current status and detailed references

The canonical current status is [`../SECURITY-STATUS.md`](../SECURITY-STATUS.md). Detailed security chapters continue from this foundation through the numbered documents in this directory, including tenant isolation, idempotency, webhook integrity, safe uploads, capabilities/egress, production policy, typed HTTP metadata, cryptographic lifecycle, supply-chain provenance, exhaustive matching, transaction outcomes, resource safety and monitoring/redaction.
