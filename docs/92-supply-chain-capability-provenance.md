<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Supply-chain capability and provenance policy

Velran treats dependency authority as reviewed release state, not as an incidental Cargo detail.

## Three-part lock

1. `Cargo.lock` fixes the resolved Rust package versions and registry checksums.
2. `SUPPLY-CHAIN-CAPABILITIES.txt` records every direct external dependency edge and the security-relevant authority it introduces: `network`, `filesystem`, `environment`, `process`, `native`, `crypto`, `parser`, or `none`.
3. `VELRAN-SUPPLY-CHAIN.lock` binds the exact hashes of both files and the accepted provenance/capability model.

Run:

```sh
./tools/supply-chain-verify.sh
```

An intentional dependency or capability change requires:

```sh
# update Cargo.toml/Cargo.lock and capability review
./tools/supply-chain-lock.sh
./tools/supply-chain-verify.sh
```

The lock refresh is an approval boundary, not an automatic post-build step. Review capability escalation before committing the regenerated lock.

## Provenance rules

The current release line accepts external Cargo packages only from the checksummed crates.io registry recorded in `Cargo.lock`. Git dependencies and alternate registries fail verification until a future provenance design explicitly supports them.

This is intentionally stricter than Cargo itself. A new dependency cannot silently appear: every direct external dependency edge must have exactly one reviewed capability row. A dependency update cannot silently alter release state: it makes `VELRAN-SUPPLY-CHAIN.lock` stale.

## Limits of this slice

This mechanism records and gates declared direct-dependency authority. It does not claim to statically infer the complete behavior of transitive third-party code. RustSec/advisory auditing and upstream review remain separate required controls.
