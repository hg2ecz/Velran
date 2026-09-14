<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Cryptographic key purpose and lifecycle

Velran distinguishes cryptographic key purpose and rotation state in the type system.

Examples:

```text
SigningKey<Webhook>
RetiringSigningKey<Webhook>
RetiredSigningKey<Webhook>
VerificationKey<Webhook>
EncryptionKey<UserData>
RetiringEncryptionKey<UserData>
RetiredEncryptionKey<UserData>
```

Key material remains classified as `Secret<T>` and cannot arrive from a web request boundary.

Webhook signing accepts only an active signing key. Signature verification may use active or retiring verification keys during rotation, but never retired keys. User-data encryption/decryption semantics are documented separately in `96-authenticated-encryption-key-lifecycle.md`.
