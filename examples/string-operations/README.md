<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# String operations

Open `/strings?text=%20Velran%20rust%20lang%20` to exercise the typed string builtins.

The example demonstrates Unicode-aware character length, trimming, case conversion, substring tests and bounded replacement. String-producing builtins are charged to the runtime allocation budget.
