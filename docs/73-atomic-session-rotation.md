<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Atomic session rotation

Velran session rotation is a credential lifecycle transition, not a normal update query.
The language requires both the old presented-token proof and a fresh issued-token proof.

```vrn
model Session {
    id: i64
    tokenHash: Secret<SessionTokenHash>
    expiresAt: DateTime
}

#[query]
fn rotateSession(
    tx: Transaction,
    tokenHash: SessionTokenHash,
    newTokenHash: Secret<SessionTokenHash>,
    expiresAt: DateTime
) -> Result<Changed, DbError>
    rotates Session by tokenHash to newTokenHash until expiresAt
sql {
    UPDATE sessions
    SET token_hash = :newTokenHash,
        expires_at = :expiresAt
    WHERE token_hash = :tokenHash
      AND expires_at > CURRENT_TIMESTAMP
}
```

At the call site the old hash must come from a validated presented bearer token:

```vrn
let oldHash = presentedTokenHash(token);
```

The replacement hash must come from a freshly issued token:

```vrn
let newToken = newSessionToken();
let newHash = tokenHash(newToken);
```

Then the transition is atomic:

```vrn
transaction db {
    let changed = rotateSession(tx, oldHash, newHash, expiresAt)?;
}
```

The compiler rejects rotation when:

- the query is not an `UPDATE`;
- the result is not `Changed`;
- the old hash does not carry presented-token evidence;
- the replacement hash does not carry fresh issued-token-hash evidence;
- the query does not replace both token hash and expiry;
- the `WHERE` clause does not match the old token hash;
- the old session expiry is not guarded against `CURRENT_TIMESTAMP`.

`tokenHash(...)` itself also requires `IssuedToken` evidence. A `Secret<SessionToken>` loaded from storage cannot be re-labelled as a fresh replacement token merely by hashing it again.

## Why cookie delivery is separate

The bundled Velran server already owns the authentication cookie (`__Host-velran_session`). A second application-level session cookie would create two competing session authorities. Secure browser delivery therefore remains a separate protocol boundary and must integrate with the existing auth/session subsystem rather than introduce a parallel cookie model.
