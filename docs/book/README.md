<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Velran book sources

The canonical English LaTeX book lives directly under `docs/book/`:

- `main.tex`
- `chapters/`

The Hungarian translation is kept separately under `docs/book/hu/`.

From the repository root:

```bash
make book       # canonical English edition
make book-hu    # Hungarian edition
make books      # both editions
```

Or build directly:

```bash
make -C docs/book pdf
make -C docs/book hu
```

Generated PDFs:

- `docs/book/velran-for-web-application-developers.pdf`
- `docs/book/hu/velran-webfejlesztoknek-hu.pdf`

The two editions should keep the same learning arc and technical contracts. English is canonical; Hungarian is maintained as a translation. Learner-facing companion applications live under `examples/`; verification-only rejection cases live under `tests/fixtures/`, and verification metadata lives under `tests/manifests/`. The focused database-backed Markdown/cache example is `examples/markdown-sql-cache/`.

### Secure Notes checkpoint discipline

The running Secure Notes project is rooted at `examples/secure-notes/`. Chapter checkpoints are mapped in `examples/secure-notes/CHECKPOINTS.tsv`; each listed `main.vrn` is independently compiler-checked by `verify.sh`. The companion-example manifest in `docs/book/CANONICAL_EXAMPLES.tsv` remains the source for focused framework examples.
