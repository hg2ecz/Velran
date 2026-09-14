<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Dependency security and reproducible builds

Velran crates use Rust edition 2024. The project intentionally does not pin a specific Rust compiler version; CI and release environments should use a maintained toolchain and validate upgrades with the full regression suite.


Velran is a server, so dependency changes are treated like source changes.

## Normal developer workflow

Use:

```sh
./verify.sh
```

After the initial lockfile bootstrap, verification uses the committed dependency graph via Cargo's `--locked` mode.

## Intentional dependency update

Do not run broad dependency updates as part of unrelated feature work. When an update is intended:

```sh
./tools/refresh-lock.sh
./verify.sh
./tools/dependency-audit.sh
```

Then review both `Cargo.toml` and `Cargo.lock` changes.

## Why Cargo.lock is committed

Velran produces deployed server binaries. Committing the lockfile makes the exact transitive graph reviewable and keeps CI, staging and production builds on the same resolved versions.

## RustSec

Release CI should install `cargo-audit` and execute `./tools/dependency-audit.sh`. The script also shows duplicate crate versions with `cargo tree --duplicates` for human review.

An audit result is evidence about known advisories in the resolved dependency graph; it is not a substitute for code review or upstream changelog review.

## Direct versus transitive dependencies

Only crates used directly by Velran belong in its manifests. Do not add a direct dependency merely to force a newer version of a transitive implementation detail. Prefer refreshing the lockfile if the parent dependency's version constraints permit the safe patch release.

## Major/minor migrations

Cryptographic, parser, TLS, database and authentication dependency migrations get their own compatibility change. They must not be hidden inside unrelated feature work.
