<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Public error boundaries

Velran separates application-visible failures from internal runtime failures.

A page or action may terminate with one of the closed public error values:

```vrn
fail badRequest;
fail notFound;
fail forbidden;
fail conflict;
```

The server maps these values to its existing safe HTTP error responses. Application code cannot use `fail internal`, `fail database`, arbitrary status codes, or arbitrary error text. Internal, database, resource-limit, and infrastructure failures remain runtime-owned and are rendered through the platform's fixed error boundary.

This keeps the happy path short while preventing accidental stack traces, SQL details, credentials, or infrastructure messages from becoming part of an HTTP response.

`fail` is terminal for a page/action body just like a normal successful return.

Security diagnostic `SEC-A10-003` rejects attempts to expose an unknown or internal error kind.
