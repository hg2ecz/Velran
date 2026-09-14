<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Native outbound example

This example exercises the native-only typed host ABI. The generated application shard remains a self-contained `std`-only `cdylib`; the trusted server owns DNS/TLS/network authority through the configured egress policy.

Configure an egress target named `catalog_api` in the server egress policy before running the route.
