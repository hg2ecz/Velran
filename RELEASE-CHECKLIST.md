<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Velran V1 release checklist

**Canonical project name:** Velran — a security-first web application language and server with Rust syntax.

## Current milestone status

The current tree has reached the **Verified Development Milestone**: `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully on a real Rust toolchain. Repository-level verification is therefore no longer pending.

The checklist below still separates this verified development baseline from a production release decision. Environment-specific deployment, recovery, secret provisioning, network/TLS topology, operator acceptance, and other target-environment evidence must be completed for the actual release environment; unchecked environment-specific items must not be inferred as passed.

## Automated workspace gate

- [ ] `./tools/check-architecture.sh` passes
- [ ] `./tools/check-clean-structure.sh` passes
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo metadata --locked --format-version 1`
- [ ] `cargo check --locked --workspace`
- [ ] `cargo test --locked --workspace`
- [ ] `./tools/supply-chain-verify.sh` passes and `VELRAN-SUPPLY-CHAIN.lock` is current
- [ ] security negative fixtures pass
- [ ] SQLite CRUD integration passes
- [ ] migration `verify` passes before and after `apply`
- [ ] local-auth administration smoke tests pass
- [ ] source examples compile through `velran-cli check`
- [ ] `RELEASE-MANIFEST.sha256` verifies successfully
- [ ] deterministic source package is produced from the exact release tree

## Environment-specific release evidence

`./verify.sh` is the repository-level automated gate. The following checks require real infrastructure or the target Linux/operator environment and must be recorded separately with date, environment and artifact hash:

- [ ] PostgreSQL integration
- [ ] MariaDB integration
- [ ] Redis session and TOTP replay integration against real Redis
- [ ] public HTTPS certificate and hostname behavior
- [ ] outbound DNS/CIDR/peer checks in a real network environment, including mixed A/AAAA resolution
- [ ] AppFs/openat2 behavior on the target Linux kernel
- [ ] cgroup/systemd resource ceilings on the target host
- [ ] slowloris/request-smuggling corpus against the target proxy/load-balancer topology
- [ ] load smoke test under approved resource limits
- [ ] no secrets, raw credentials, tokens, request bodies, or raw principal/source identifiers appear in security logs
- [ ] authentication-abuse and security-alert thresholds are exercised against the target Redis/logging topology
- [ ] systemd/container hardening profile is reviewed
- [ ] backup restore drill and upgrade/rollback rehearsal in an isolated production-like environment

## Deployment gate

- [ ] application source passes `velran-cli check`
- [ ] production config passes `velran-server --check-config` in the target environment
- [ ] application `production { ... }` policy matches the effective HTTPS/HSTS/database-TLS/origin/cookie topology
- [ ] named outbound integration targets resolve to reviewed host/port/CIDR/TLS policy
- [ ] release directory is immutable/read-only to the service
- [ ] release artifact SHA-256 and migration set are recorded
- [ ] migration credential is unavailable to the application service
- [ ] backup exists and restore readiness is confirmed before migration apply
- [ ] service-manager hard ceilings match approved operator policy
- [ ] rollout completes only after liveness and readiness succeed
- [ ] startup logs, structured security events, burst alerts, audit events and resource-profile startup audit are reviewed
- [ ] critical/idempotent transaction paths have explicit `Committed` / `RolledBack` / `CommitUnknown` handling

## Recovery and upgrade gate

- [ ] production state inventory covers application DB, local-auth DB, AppFs data root and Redis role
- [ ] RPO/RTO, retention, encryption and access policy are documented
- [ ] consistent DB + AppFs recovery-point strategy is documented
- [ ] local-auth DB is backed up separately as sensitive authentication material when enabled
- [ ] release/source/migration/config hashes are recorded without secret values
- [ ] latest backup has a successful isolated restore-drill record
- [ ] restore drill passes config check, source check, `migrate verify`, live/ready and application smoke tests
- [ ] schema compatibility is classified before rollout
- [ ] app-only rollback is used only with a backward-compatible schema
- [ ] no automatic reverse/down migration is part of the rollback path
- [ ] destructive restore has an explicit recovery point and accepted data-loss window
- [ ] Redis loss behavior is accepted: session reset, cache rebuild and rate-limit window reset

## IPv6 egress evidence

- [ ] mixed A/AAAA resolution is exercised
- [ ] one denied DNS candidate rejects the whole outbound request
- [ ] IPv6 connected peer is rechecked against configured CIDRs
- [ ] IPv4-mapped IPv6 follows IPv4 CIDR policy
- [ ] TLS hostname verification succeeds over an allowed IPv6 peer

## Final packaging gate

- [ ] `Cargo.lock` is present in the exact release workspace and reviewed
- [ ] `SUPPLY-CHAIN-CAPABILITIES.txt` is reviewed for every direct external dependency edge
- [ ] `VELRAN-SUPPLY-CHAIN.lock` seals the exact `Cargo.lock` + capability policy state
- [ ] `./verify.sh` does not generate or modify `Cargo.lock`
- [ ] `cargo metadata --locked`, `cargo check --locked --workspace`, and `cargo test --locked --workspace` pass
- [ ] `RELEASE-MANIFEST.sha256` is generated from the exact release source tree
- [ ] `sha256sum -c RELEASE-MANIFEST.sha256` passes before packaging
- [ ] `tools/package-release.sh` produces the deterministic source artifact
- [ ] source artifact SHA-256 is recorded with release evidence
- [ ] `RELEASE-NOTES-V1.0.md` is published with the artifact
- [ ] no post-freeze feature change is included without explicit release-blocker justification

## Final repository stabilization

- [x] Native artifact manifest v4 carries a runtime contract fingerprint.
- [x] Loaded shards verify ABI + language/security/EIR/codegen contract at activation.
- [x] Native cache namespace bumped for the new contract.
- [x] Release shard compilation uses O3 + one codegen unit + ThinLTO.
- [x] Interactive compilation remains O2 without LTO.
- [x] Request-time native dispatch performs no symbol resolution.
- [x] Master structural/security gate passes.
- [x] Standalone static gate sweep has zero failures.
- [ ] Run Cargo/rustc positive-example compilation on the deployment/build machine.
- [ ] Record same-machine FFT4096 x10000 and typed-data-fastpaths release baselines.


- Run `VELRAN_REQUIRE_RUST_TOOLCHAIN=1 tools/check-positive-examples.sh` so release CI cannot silently skip real Rust compilation.

## Deployment build identity

- Run `velran-server --print-build-id` from the exact release binary.
- Pin the returned 64-character ID as `server.expected_build_id` in the production server config.
- Run `--check-config` after updating the pin and before switching traffic.
- A build-id mismatch must fail before application compilation or listener startup.

## Post-release compile-clean gate

- [ ] `VELRAN_REQUIRE_RUST_TOOLCHAIN=1 ./tools/check-release-compile-clean.sh` passes on the release build host.
- [ ] Workspace `cargo check` and `cargo test` are warning-free under `-Dwarnings`.
- [ ] Representative generated native shards compile as real `cdylib` files with direct rustc.

## Native cache proof

- [ ] `VELRAN_REQUIRE_RUST_TOOLCHAIN=1 ./tools/check-native-cache-reuse-integration.sh` passes.
- [ ] Cold bootstrap invokes rustc; identical warm bootstrap invokes rustc zero times.
- [ ] Warm startup emits `native_binary_cache_hit` for native shards.

## Performance regression gate

- [ ] Record a same-machine baseline with `tools/performance-regression.sh record`.
- [ ] Candidate release passes `tools/performance-regression.sh check` with the approved relative threshold.
- [ ] FFT correctness marker passes and benchmark samples are collected after warm-up.

## Deterministic adversarial/fuzz gate

- [ ] Compiler/parser/verifier deterministic mutation corpus completes without panic.
- [ ] HTTP request-head smuggling corpus rejects every fixed dangerous case.
- [ ] HTTP deterministic mutation corpus completes without panic.

## Reproducibility and crash recovery

- [ ] Run `SOURCE_DATE_EPOCH=<approved epoch> VELRAN_REQUIRE_RUST_TOOLCHAIN=1 ./tools/check-reproducible-release-recovery.sh`.
- [ ] Two packages from the exact same tree and epoch have identical SHA-256 hashes.
- [ ] Release server returns a stable 64-hex `--print-build-id`.
- [ ] Abandoned compiler/cache staging entries are recovered without deleting live/unknown entries.

### Module/import surface

- [ ] `tools/check-module-imports.sh` passes.
- [ ] `examples/module-imports/main.vrn` passes `velran-cli check` in release CI.
- [ ] Nested private modules fail closed across sibling namespaces; `pub mod` opens only the module boundary, not private items or web authority.
- [ ] Narrow package-root `pub use path as alias;` re-exports pass; wildcard imports, direct item re-exports, private-module re-exports, private transitive-dependency re-exports, and bare-symbol injection remain rejected.
