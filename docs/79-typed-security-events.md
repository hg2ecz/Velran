<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed security events

Velran security events are declared once and tied to a model:

```vrn
security event RoleGranted for User;
```

Inside a transaction, emit the event with a typed object identifier:

```vrn
security RoleGranted user.id;
```

Optional public, non-sensitive state transitions may be recorded:

```vrn
security RoleChanged user.id from oldRole to newRole;
```

`Secret<T>` and `Sensitive<T>` values remain forbidden in security event fields. Critical operations may require a specific event:

```vrn
critical RoleChange {
    permission UserAdmin
    mfa
    transaction
    audit RoleGranted
}
```

A different business audit or a different security event does not satisfy that contract. This keeps the normal application code short while making security-sensitive state changes observable by construction.
