<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed credentials and password primitives

Velran deliberately exposes high-level password operations instead of raw cryptographic algorithms.

```vrn
let hash = passwordHash(password);
let valid = passwordVerify(account.passwordHash, password);
```

`passwordHash(Password)` accepts a validated purpose-typed password and produces statically classified `Secret<PasswordHash>` data. The runtime uses Argon2id with a fresh random salt and policy-owned parameters. The language does not expose algorithm, salt, or cost knobs in ordinary application code.

`passwordVerify(Secret<PasswordHash>, Password)` requires the exact password-hash purpose and a validated purpose-typed password input. Its result is a public `bool`; the secret classification does not leak through the comparison result.

These operations do not create authorization evidence. Loading an account and checking or changing its password remains subject to the normal authentication/authorization contracts.

Raw password hashing primitives are intentionally absent from the normal Velran builtin surface. Nominal domain types such as `PasswordResetToken`, `ApiToken`, and `SessionToken` should remain distinct even when they share a String representation.
