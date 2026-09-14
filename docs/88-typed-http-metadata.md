<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed HTTP metadata

Server response metadata is a typed boundary rather than arbitrary `(String, String)` mutation.

Implemented domain/boundary types include:

- closed `HeaderName` values for platform-owned response headers;
- bounded/control-character-safe header values;
- `MediaType`;
- `FileName`;
- `ContentDisposition`.

Header values reject CR/LF and unsafe control bytes. Invalid response metadata fails closed before response bytes are emitted. Safe filenames are bounded single segments: path separators, traversal forms, dotfile-style unsafe names and response-splitting characters are rejected.

Persisted idempotency responses are revalidated when replayed; serialized header strings do not bypass the typed boundary.
