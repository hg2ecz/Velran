<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Repository test assets

This directory contains repository-level verification assets that are not user-facing examples.

- `fixtures/security/`: security-negative sources that must be rejected by the compiler.
- `fixtures/negative/`: other language/framework rejection fixtures.
- `manifests/example-entrypoints.txt`: one canonical entrypoint for every Velran source example directory.
- `manifests/native-pending.txt`: generated compatibility/status inventory used by native-status checks.
- `NATIVE_STATUS.md`: verification-oriented native coverage notes.

The separation is intentional: `examples/` is for code worth learning from; `tests/` is for code or metadata whose primary purpose is verification.