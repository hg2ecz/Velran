<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Velran examples

This directory contains user-facing examples: tutorials, feature demonstrations, starter applications, deployment/configuration samples, and performance demonstrations that are useful to read or adapt.

The examples are also compiled by the verification suite where practical, but they are not test fixtures. The canonical example entrypoint list lives in `tests/manifests/example-entrypoints.txt`.

Compiler rejection cases and security-negative fixtures live under `tests/fixtures/`, not here. Verification metadata lives under `tests/manifests/`.

A directory may remain here even when `verify.sh` checks it, as long as it is independently useful as documentation or teaching material.