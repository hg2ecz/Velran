<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Route budget profiles

Velran applies the platform request budget to every request automatically. Application code does not need to opt in to instruction and allocation limits.

A route that is intentionally more expensive or must have a tighter concurrency/runtime profile may request an operator-defined named profile:

```vrn
#[page]
fn report(ctx: PageContext) -> Result<Html, PageError> {
    let total = 40 + 2;
    return Ok(html {<p>{{ total }}</p>});
}

route report GET "/report"
    auth user
    budget report
    => report;
```

`budget report` does not define numeric limits in application source. The trusted server resource-profile configuration owns `max_instructions`, `max_allocated_bytes`, and `max_concurrent`. This prevents application code from silently weakening production ceilings.

## Security properties

- Every request has a platform default budget even when the route contains no `budget` clause.
- A named route budget remains inside the request hard ceiling.
- Unknown profiles fail server configuration/preflight because route budget uses are recorded in the compiled program.
- `budget default` is rejected (`SEC-A10-002`); the default is implicit and should not create ceremony.
- A route-level budget and a handler-level `with resource ...` block cannot be combined (`SEC-A10-001`). This keeps one clear resource authority for the handler and prevents an inner profile from accidentally bypassing the intended route ceiling.
- Named profiles also carry their existing concurrency semaphore, so expensive routes can be isolated from normal traffic.

## Developer experience

Normal routes remain short:

```vrn
route home GET "/" public => home;
```

Only exceptional routes name a profile:

```vrn
route export GET "/export" auth user budget export => export;
```

The developer expresses intent (`export`); production operators own the actual values. This keeps resource security reviewable without turning each route into a list of numeric limits.

## OWASP relevance

This is primarily an A10:2025 Mishandling of Exceptional Conditions / resource-exhaustion defense. It complements HTTP body/header limits, bounded input types, runtime allocation/instruction accounting, server request timeouts, and process/cgroup ceilings rather than replacing them.
