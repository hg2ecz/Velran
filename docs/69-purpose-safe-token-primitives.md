<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Purpose-safe token primitives

Velran issues protocol tokens through purpose-specific primitives instead of raw random-string APIs:

```vrn
let session = newSessionToken();
let reset = newPasswordResetToken();
let csrf = newCsrfToken();
```

Each result is statically classified as a secret credential:

- `newSessionToken()` -> `Secret<SessionToken>`
- `newPasswordResetToken()` -> `Secret<PasswordResetToken>`
- `newCsrfToken()` -> `Secret<CsrfToken>`

The runtime generates 256 bits of cryptographically secure randomness and encodes it as 64 lowercase hexadecimal characters. Applications cannot select a weaker RNG or token length.

## Verification

`tokenMatches(storedHash, presented)` requires:

1. `storedHash` to be a purpose-specific secret token hash (`Secret<...TokenHash>`);
2. `presented` to have the exact same credential purpose;
3. the presented token to have crossed a validating request boundary.

The supported verification purposes in this layer are `SessionToken`, `PasswordResetToken`, and `CsrfToken`. A reset token cannot be compared as a session token merely because both use the same runtime string representation.

The runtime SHA-256 hashes the presented token and compares the fixed-size token hashes with a constant-time comparison primitive. Session, password-reset, and CSRF request values are constrained to the fixed 64-character generated-token representation.

## Deliberately no generic delivery escape hatch

Generated tokens remain blocked from generic HTML/JSON rendering. Before persistence they must flow through `tokenHash(...)` into an explicit `Secret<TokenHashPurpose>` sink; delivery to cookies, reset links, or CSRF form fields must use dedicated protocol boundaries. Those boundaries are intentionally separate so that adding token generation does not reopen credential leakage through generic strings.

## Security model

Token purpose, sensitivity, and trust remain independent facts:

```text
Secret<SessionToken>       issued bearer credential
Secret<SessionTokenHash>   persisted verification credential
Validated<SessionToken>    presented request credential
```

`tokenMatches` verifies a presented token against its persisted hash; it does not grant authorization and does not prove token expiry or single-use consumption. Expiry and consume-once semantics belong to the next lifecycle layer.
