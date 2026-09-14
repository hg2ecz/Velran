<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Transaction outcome semantics

Critical database operations cannot safely collapse every failure into a generic database error.

Captured transactions use a closed outcome model:

- `Committed`: commit acknowledgement was received;
- `RolledBack`: non-commit/rollback is known;
- `CommitUnknown`: commit or rollback acknowledgement is unavailable and retry must not be assumed safe.

Example:

```velran
let outcome = transaction db {
    charge(tx)?;
};

match outcome {
    Committed => { return Ok(json(true)); }
    RolledBack => { fail conflict; }
    CommitUnknown => { fail conflict; }
}
```

For critical operations that require both transaction and idempotency, the compiler requires transaction outcome capture and exhaustive handling. This prevents an unknown commit state from being reduced to a normal retryable failure.
