<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Secret consumer contracts

Velran treats `Secret<T>` as a compile-time data classification, not as a cosmetic alias.
A secret value may only cross a sink that explicitly declares that it consumes `Secret<T>`.

## Explicit query sinks

A query that intentionally consumes a secret must say so in its signature:

```vrn
#[query]
fn lookupByToken(
    db: Db,
    token: Secret<String>
) -> Result<Option<Credential>, DbError> sql {
    SELECT id, token FROM credentials WHERE token = :token
}
```

Passing `Secret<String>` to a plain `String` parameter is rejected (`SEC-DATA-006`).
Passing ordinary `String` data to a `Secret<String>` parameter is also rejected (`SEC-DATA-007`).
This makes the annotation a two-way contract: a secret cannot leak into an unclassified sink, and unclassified data cannot impersonate a secret capability.

Nominal type identity remains independent of sensitivity. `Secret<ApiToken>` still requires the exact `ApiToken` domain type as well as secret classification.

## Web handler parameters are not secret capabilities

Request parameters cannot be declared `Secret<T>` or `Sensitive<T>` in a page/action signature (`SEC-DATA-009`). Request data is a trust-boundary input. Sensitive or secret values must come from an explicitly classified model field or another trusted capability.

## Audit boundary

Business audit records reject `Sensitive<T>` and `Secret<T>` values (`SEC-DATA-008`) for object IDs and before/after values. Audit stable identifiers, status/enum transitions, or deliberately redacted public data instead.

This rule prevents long-lived audit storage from becoming an accidental credential or PII sink.

## Design rule

Velran follows this rule for future secret-consuming APIs:

> A secret can only be consumed by an API whose type contract explicitly says that it consumes a secret.

Logging, debugging, URLs, response bodies, public projections and audit records are not secret consumers. Outbound authentication and cryptographic primitives should expose narrow secret-consuming capabilities rather than generic string APIs.
