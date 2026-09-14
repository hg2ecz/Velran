<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Token hashes at rest

Velran bearer tokens are issued as purpose-specific secret credentials, but persistent verification state uses a separate purpose-specific hash type.

```vrn
let token = newSessionToken();
let storedHash = tokenHash(token);
```

The static types are:

```text
newSessionToken()        -> Secret<SessionToken>
tokenHash(sessionToken)  -> Secret<SessionTokenHash>
```

Equivalent hash purposes exist for password-reset and CSRF tokens:

- `SessionTokenHash`
- `PasswordResetTokenHash`
- `CsrfTokenHash`

The raw and hashed purposes are not interchangeable.

## Persistence contract

Persist the hash, not the bearer token:

```vrn
#[query]
fn storeSession(
    tx: Transaction,
    tokenHash: Secret<SessionTokenHash>
) -> Result<(), DbError> sql {
    INSERT INTO sessions(token_hash) VALUES(:tokenHash)
}
```

A generated raw token cannot be passed to this sink without first calling `tokenHash(...)`, and a request-presented token cannot be promoted into a persistent token hash.

## Verification contract

`tokenMatches(storedHash, presented)` is the equality-only primitive and is reserved for CSRF token hashes. For session and password-reset tokens use `tokenActive(storedHash, presented, expiresAt)`. Both verification forms require:

1. a secret purpose-specific token hash as the first argument;
2. a validated raw token of the matching purpose as the second argument.

```vrn
let valid = tokenActive(session.tokenHash, presentedSessionToken, session.expiresAt);
```

For example, `Secret<SessionTokenHash>` can only be checked against `SessionToken`. It cannot be checked against a reset or CSRF token.

The runtime hashes the presented token with SHA-256 and compares the fixed-size hexadecimal hashes using a constant-time comparison primitive. The generated bearer token contains 256 bits of random entropy, so the persisted hash does not expose a practically searchable low-entropy credential space.

## Security boundary

Hashing is intentionally one-way at the language surface. There is no token-hash-to-token conversion and no generic credential rendering API.

`tokenHash(...)` accepts only an issued secret token. This matters because hashing an arbitrary request string must not become a way to manufacture a trusted persistence credential.

## Lifecycle scope

This layer establishes safe persistent token identity. Expiry-aware session/reset verification is provided by `tokenActive(...)`. It does not yet prove:

- single-use consumption;
- session rotation;
- revocation;
- protocol delivery.

Those lifecycle states can build on the hash identity without storing the bearer token itself.
