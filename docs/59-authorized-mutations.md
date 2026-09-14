<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Authorized mutations

Velran treats object identity as part of the authorization proof for `UPDATE` and `DELETE` queries.

A mutating query declares its protected model and key:

```velran
#[query]
fn updateArticle(
    tx: Transaction,
    id: i64,
    title: String
) -> Result<(), DbError> mutates Article by id sql {
    UPDATE articles SET title = :title WHERE id = :id
}
```

`UPDATE` and `DELETE` without a mutation contract are rejected with `SEC-A01-006`.

The call must pass a key that comes from an authorized instance of the declared model:

```velran
let article = articleById(db, id)?;
authorize article owner authorUsername or role Editor;

transaction db {
    updateArticle(tx, article.id, title)?;
}
```

Passing the request parameter directly is rejected with `SEC-A01-005`:

```velran
updateArticle(tx, id, title)?; // rejected
```

Unchanged local aliases preserve the proof:

```velran
let articleId = article.id;
updateArticle(tx, articleId, title)?; // accepted
```

Transformations intentionally destroy the proof:

```velran
let otherId = article.id + 1;
updateArticle(tx, otherId, title)?; // rejected
```

A proof from another model is not interchangeable, even when the key type is identical.

## Shared objects

For application objects that are intentionally writable by every authenticated principal, use an explicit policy:

```velran
authorize product authenticated;
```

This still requires an authenticated route and produces object-specific proof without inventing a fake owner field.

## Fresh objects

A model returned from an `INSERT ... RETURNING` query is treated as fresh authority inside the same flow. This keeps create-then-update transaction code concise while preventing request-controlled identifiers from becoming mutation authority.
