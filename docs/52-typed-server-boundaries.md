<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed server configuration and TLS errors

The server keeps dynamic error erasure at the outer application boundary only. Internal leaf modules that validate configuration, host names, static URL prefixes, secret files, or TLS material return typed errors.

## Why

Configuration and TLS failures are operationally different from application request failures. Returning `Box<dyn Error>` or plain strings from every helper hides that distinction and makes tests depend on error text.

The server now uses dedicated types for these leaf boundaries:

- `ServerConfigError` for server/domain config loading and validation;
- `TlsConfigError` for certificate/key loading and rustls configuration;
- `PublicHostError` for canonical public-host validation;
- `StaticPrefixError` for static URL-prefix validation;
- `SecretFileError` for unreadable or empty secret files.

I/O and TOML parse failures preserve their original source error and the relevant file path. Validation errors retain stable categories while still producing human-readable startup messages.

## Application boundary

`startup::run()` intentionally remains an aggregation boundary because it composes errors from database, Redis, authentication, storage, resource limits, TLS, and configuration crates. It may erase those heterogeneous errors at the top level, but leaf modules should not introduce new `Box<dyn Error>` APIs.

This keeps the implementation pragmatic: typed errors where callers can act on categories, dynamic aggregation only where the process is about to log and exit.
