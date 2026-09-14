<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Expiry-aware token verification

Session and password-reset credentials have a time-bounded lifecycle. Velran therefore does not allow equality-only verification for these token purposes.

Use `tokenActive(storedHash, presentedToken, expiresAt)`:

```vrn
model Session {
    id: i64
    tokenHash: Secret<SessionTokenHash>
    expiresAt: DateTime
}

let valid = tokenActive(session.tokenHash, presentedSessionToken, session.expiresAt);
```

The compiler requires:

1. `Secret<SessionTokenHash>` with `SessionToken`, or `Secret<PasswordResetTokenHash>` with `PasswordResetToken`;
2. a validated presented token of the matching purpose;
3. an explicit `DateTime` expiry value.

`tokenMatches(...)` is intentionally reserved for CSRF token equality. Using it with session or password-reset hashes is a compile-time security error (`SEC-A07-004`), so applications cannot accidentally omit expiry checks for those credentials.

At runtime `tokenActive(...)` hashes the presented bearer token with SHA-256, compares the fixed-size hashes in constant time, and returns `false` when `expiresAt <= now`.

This primitive proves only two facts at runtime: the bearer value matches the persisted hash, and the credential has not expired. The returned `bool` is deliberately not an authorization or consume-once proof. Password-reset consumption and session rotation/revocation are separate lifecycle transitions and must not be inferred from a boolean equality result.
