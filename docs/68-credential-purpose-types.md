<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Credential purpose types

Velran credential values have first-class purpose identities. The built-in types are:

- `Password`
- `PasswordHash`
- `ApiToken`
- `SessionToken`
- `PasswordResetToken`
- `CsrfToken`
- `CryptoKey`

They use an erased String representation at runtime, but they are not interchangeable at compile time. A `Secret<ApiToken>` cannot be passed where `Secret<PasswordHash>` is required, even though both are represented as strings.

Password input is declared directly at the web boundary:

```vrn
route change POST "/account/password"
    form password<Password>
    auth user => change;
```

`Password` decoding enforces the platform password length policy before the handler executes. Generic `String` input is deliberately not accepted by password cryptography:

```vrn
let hash = passwordHash(password); // password: Password
```

`passwordHash(Password)` returns `Secret<PasswordHash>`. `passwordVerify` requires exactly `Secret<PasswordHash>` and `Password`:

```vrn
let valid = passwordVerify(account.passwordHash, password);
```

Credential-purpose values cannot be rendered, serialized, exposed through a generic model projection, or placed in business audit fields. Protocol-specific operations must provide explicit, narrow boundaries for credentials. This keeps accidental credential disclosure out of the normal HTML/JSON/data-flow surface.

The runtime representation is intentionally erased: purpose types add compile-time meaning without allocating wrapper objects. Token generation, expiry, single-use reset semantics, session issuance, and CSRF-specific disclosure are separate lifecycle concerns and should use dedicated primitives rather than weakening the generic credential boundary.
