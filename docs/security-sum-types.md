<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# First-class sum types and exhaustive match

Velran enums are first-class, closed sum types. A `match` over a sum value must enumerate every declared variant exactly once.

```vrn
enum CommitState {
    Committed
    RolledBack
    CommitUnknown
}

#[page]
fn outcome(ctx: PageContext, state: CommitState) -> Result<Json, PageError> {
    match state {
        Committed => { return Ok(json("committed")); }
        RolledBack => { return Ok(json("rolledBack")); }
        CommitUnknown => { fail conflict; }
    }
}
```

Security invariants:

- match subjects must be closed sum/enum values;
- every variant must appear exactly once;
- unknown variants are rejected;
- wildcard arms are intentionally unsupported for security-critical state machines;
- adding a new variant makes existing matches fail compilation until the new state is handled;
- match arms keep normal compiler effect, authorization, response and resource checks;
- runtime dispatch verifies the enum identity as well as the variant before executing an arm.

Diagnostics:

- `SEC-A10-020`: match subject is not a sum type;
- `SEC-A10-021`: unknown variant;
- `SEC-A10-022`: duplicate variant;
- `SEC-A10-023`: non-exhaustive match.

This is the language foundation for explicit transaction outcomes such as `Committed | RolledBack | CommitUnknown`.
