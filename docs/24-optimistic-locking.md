<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Optimistic locking and concurrent edits

When two editors open the same record, a normal `UPDATE ... WHERE id = :id` can silently let the last save overwrite the first. Velran provides `Changed` for this case.

## Model

```velran
model Article {
    id: i64
    title: String
    version: i64
}
```

The edit page includes the current `version` as a typed form value. The update query checks it:

```velran
#[query]
fn updateArticle(tx: Transaction, id: i64, title: String, version: i64) -> Result<Changed, DbError> mutates Article by id sql {
    UPDATE articles
    SET title = :title, version = version + 1
    WHERE id = :id AND version = :version
}
```

`Changed` means **exactly one affected row**. One row is success, zero rows map to HTTP `409 Conflict`, and more than one affected row is treated as a database error because the mutation violated its own invariant. `Changed` is mutation-only and cannot use `RETURNING` in V1.

If another request already changed the row, the old version no longer matches, so this optimistic-locking update affects zero rows and returns `409 Conflict`. The application should show a conflict page or redirect the editor back to a fresh edit view. Do not automatically overwrite the newer value.

## When to use it

Use optimistic locking on records that humans can edit concurrently: articles, CMS pages, tickets, customer records, invoices before finalization, configuration records, and similar business objects.

It is usually unnecessary for append-only events or counters that already use atomic database operations.
