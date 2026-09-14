<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Module namespaces

This example demonstrates Velran's V1 module namespace model.

`main.vrn` declares the complete source graph explicitly:

```vrn
mod catalog;
mod catalog::queries;
mod catalog::pages;
```

The mapping is application-root-relative and canonical:

```text
catalog.vrn          -> catalog
catalog/queries.vrn  -> catalog::queries
catalog/pages.vrn    -> catalog::pages
```

Loading a module does not inject its declarations into a global namespace. Cross-module references therefore stay qualified: `catalog::Product`, `catalog::queries::recent(...)`, and `catalog::pages::index`.

Within one module, its own declarations may still be referenced by their short local names. Routes remain application-global HTTP identifiers and are separate from code namespaces.

Check it with:

```bash
cargo run --locked -q -p velran-cli -- check examples/module-namespaces/main.vrn
```
