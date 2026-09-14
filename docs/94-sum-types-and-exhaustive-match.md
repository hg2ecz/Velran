<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Closed sum types and exhaustive match

Velran's existing enum representation is a first-class closed sum type for control-flow purposes.

```velran
enum CommitState {
    Committed
    RolledBack
    CommitUnknown
}

match state {
    Committed => { ... }
    RolledBack => { ... }
    CommitUnknown => { ... }
}
```

Compiler invariants:

- only a sum/enum value can be matched;
- unknown variants are rejected;
- duplicate arms are rejected;
- every declared variant must be handled;
- no wildcard arm hides future variants;
- return contracts, effects, authorization, critical-operation checks and resource analysis traverse every arm.

Adding a new variant therefore turns old incomplete handling into a compile-time error rather than silently selecting a fallback branch.
