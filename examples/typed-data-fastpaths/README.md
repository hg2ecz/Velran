<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed data fastpaths benchmark

Exercises the direct typed string, `Vec<String>`-style list, and `BTreeMap<String,String>` local paths for 10,000 iterations. Setup is outside the timed region. The checksum keeps the work observable.
