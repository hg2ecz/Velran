<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Effect / capability foundation

Velran records security-relevant handler effects in compiler metadata instead of requiring normal application code to repeat effect lists.

The first effect set is:

- `db.read`
- `db.write`
- `security.audit`

A handler receives database authority only through an explicit `db: Db` capability parameter. Writing the identifier `db` at a query call or `transaction db` site does not create authority.

For example, this remains the normal happy path:

```vrn
#[page]
fn article(ctx: PageContext, db: Db, id: ArticleId) -> Result<Json, PageError> {
    let article = byId(db, id)?;
    return Ok(json(expose(article, id, title)));
}
```

The compiler infers `db.read` for the handler. No effect annotation is required.

This is rejected with `SEC-EFFECT-001`:

```vrn
#[page]
fn article(ctx: PageContext, id: ArticleId) -> Result<Json, PageError> {
    let article = byId(db, id)?;
    return Ok(json(expose(article, id, title)));
}
```

The same rule applies to `transaction db`: database write authority must originate from the handler's explicit `db: Db` capability.

## Why inference first

The effect system is intended to remove ambient authority, not add boilerplate. The compiler therefore infers effects from the typed AST and stores them on `PageFunction` / `ActionFunction`. Later iterations can expose these effects in security reports, IDE hovers, module-boundary contracts, outbound integrations, filesystem capabilities, and secret capabilities without changing the normal handler syntax.
