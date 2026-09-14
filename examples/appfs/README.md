<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# AppFs local test root

Development example:

```text
data_root = examples/appfs
fs_mode = rwc
```

The `uploads/` directory is intentionally pre-created. Application paths are relative to this root.
