<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Secure Notes

Canonical multi-file running example for the English and Hungarian Velran web-developer books.

The current baseline intentionally stays small: authenticated note listing, create, view, edit, delete, explicit ownership authorization, bounded form input, CSRF-bearing forms, transactions, and typed `Draft`/`Published` domain state. Later book checkpoints describe how to extend this baseline with optimistic locking, publication workflow, media, JSON APIs, observability, deployment, and recovery concerns without weakening these invariants.

The release verification checks `main.vrn`; its `mod` declarations load the companion modules.

## Files

- `main.vrn` - module root
- `models.vrn` - typed domain model and publication state
- `queries.vrn` - bounded reads and declared mutations
- `pages.vrn` - authenticated HTML views with object ownership checks
- `actions.vrn` - transactional create/update/delete handlers
- `migrations/0001_init.sql` - baseline database schema

The book deliberately evolves this baseline in later checkpoints rather than hiding all advanced features in the first chapter.

## Compiler-checked chapter checkpoints

The `checkpoints/` directory contains frozen, independently compiler-checkable snapshots for the main learning milestones. `CHECKPOINTS.tsv` is the machine-readable map used by verification and documentation tooling.

The snapshots deliberately keep the baseline security invariants unchanged unless a later milestone is explicitly evolved and its guard is updated. Chapter-specific framework surfaces that are better demonstrated by an existing canonical example are listed as verified companions in the checkpoint README instead of being duplicated into Secure Notes.
