<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Wiki example

Minimal multi-concern wiki example used by the web-developer book. It demonstrates a typed model, a named form, typed SQL, Markdown rendering, an authenticated edit action, and a transaction.

Compile-check it with:

```sh
velran-cli check examples/wiki/main.vrn
```
