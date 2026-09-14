<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Nominal domain types

Velran domain types are nominal, not aliases.

```vrn
type UserId = i64 { range 1 999999999; }
type ArticleId = i64 { range 1 999999999; }
```

Even though both use `i64` at runtime, they are different compile-time types. This is rejected:

```vrn
#[query]
fn loadUser(db: Db, id: UserId) -> Result<Option<User>, DbError> sql {
    SELECT id FROM users WHERE id = :id
}

let user = loadUser(db, articleId)?;
```

The compiler reports `SEC-TYPE-001` and names both domain types. A raw `i64` literal is not silently promoted to `UserId` either.

Nominal identity is preserved across route parameters, handler parameters, model fields, query parameters, local aliases, typed redirects, and typed HTML route helpers. This prevents accidental cross-object identifier use and strengthens Velran's authorization-proof model.

Representation-consuming safe operations remain ergonomic. A `Username = String { ... }` can be HTML-escaped, measured with `len`, split with `splitBounded`, or passed to other safe string operations. These operations consume its `String` representation and may return a primitive result; they do not implicitly manufacture a new validated `Username`.

String domain types also receive a fail-closed `length 0 4096` constraint when the declaration only provides another constraint such as `pattern`. Application-specific shorter bounds should still be declared when known.

At runtime, domain values use their base scalar representation. The nominal identity exists in the compiled program's type contract, not as a heap wrapper, so the stronger type safety does not add per-value allocation overhead.
