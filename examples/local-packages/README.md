<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Local packages
Security-first local path library example. `velran.toml` permits only explicit relative path dependencies. Libraries get a stable package namespace, remain private-by-default, and may expose only ordinary pure/API code; web/server authority stays in the application. Remote registry/git/build scripts and transitive dependencies are intentionally deferred.
