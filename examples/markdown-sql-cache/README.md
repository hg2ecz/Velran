<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# SQL-backed Markdown rendering with optional HTML response cache

This example shows the recommended Velran pattern for database-backed Markdown:

1. keep the original Markdown source in SQL;
2. load the row with a typed `#[query]` using a bound `:id` parameter;
3. render the `String` with the source-level `commonmark::render(...)` module;
4. optionally cache the finished public HTML response with the normal route cache.

The application is in [`main.vrn`](main.vrn). Database bootstrap examples are provided for SQLite, PostgreSQL, and MariaDB.

## Routes

`GET /documents/:id<i64>` is uncached. Each request reads the current row and renders the Markdown.

`GET /documents-cached/:id<i64>` uses:

```velran
cache public ttl 120
```

The route parameter participates in the cache identity, so different document IDs do not share one HTML response. Remove the cache clause when every request must observe the current database value immediately.

## Security properties

The SQL statement uses a named bind (`:id`), not string-built SQL. Markdown is rendered by the Velran source module `commonmark.vrn`; the engine has no Markdown-specific directive. The module can only construct typed `SafeHtml` through the engine's generic safe HTML builders. Raw HTML in Markdown is emitted as escaped text, and unsafe link schemes are rendered as text rather than trusted markup.

The public cache should be used only when the generated page is public and independent of user/session/request secrets. Velran's compiler performs cache-safety checks for public cached routes.

## Compiler/language boundary

The CommonMark source module is compiled as ordinary Velran. Its use of verified pure helpers, scalar parameters, branching, recursion and string borrows is part of the general language contract described in [`docs/58-verified-pure-functions.md`](../../docs/58-verified-pure-functions.md), not a Markdown-specific engine path.

## Cache freshness

The route cache stores the rendered public response, not a second HTML copy in the database. This keeps Markdown as the source of truth and keeps HTML generation under the framework security policy.

If the document can be edited by the application, the write route should invalidate the cached read route. If another process modifies the SQL table directly, cached output can remain visible until the configured TTL expires, so choose the TTL accordingly or avoid route caching.

## Database bootstrap

For SQLite:

```sh
sqlite3 app.db < examples/markdown-sql-cache/sqlite.sql
```

Equivalent schema files are included as `postgresql.sql` and `mariadb.sql`.

## Source-level module

`main.vrn` includes the sibling source file with `mod commonmark;`. Copy `commonmark.vrn` with the application when adapting this example.
