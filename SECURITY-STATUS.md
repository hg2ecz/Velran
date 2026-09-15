<!-- VELRAN-DOC-STATUS: 2026-09-15 -->
> **Documentation status (2026-09-15):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Velran security status

**Canonical project name:** Velran — a security-first web application language and server with Rust syntax.

Velran is a security-oriented web programming language/runtime. Its security model follows one rule consistently:

> If a security property can be proven statically, make it a compiler invariant. If it cannot, enforce a secure runtime/platform default. If neither is possible, fail closed with a stable security diagnostic.

This document is the canonical high-level status of the implemented security model. Detailed language/runtime behavior lives in `docs/`.

## Current security architecture

### Access control and authority

Implemented:

- every route has an explicit access policy; there is no implicit public route;
- object authorization produces flow-sensitive compiler evidence;
- protected UPDATE/DELETE operations require mutation authorization evidence;
- named function permissions and MFA elevation are compiler/runtime contracts;
- critical operations compose permission, MFA, transaction, audit and idempotency requirements;
- tenant membership is platform-owned authority and scoped-model queries require active-tenant proof;
- effect/capability metadata prevents ambient DB and outbound-network authority;
- named outbound integrations do not expose arbitrary host/IP/port authority to Velran source.

### Input, injection and browser boundaries

Implemented:

- typed SQL bind contracts; raw/interpolated SQL is rejected;
- structured HTML/template output and sensitivity-aware rendering;
- compiler-checked typed route redirects rather than string-built URLs;
- nominal/domain types with explicit validation/refinement;
- bounded request strings and bounded request collections;
- staged file uploads with byte-authoritative image inspection before publication;
- verified webhook routes with raw-body signature verification and replay protection;
- typed HTTP response metadata, media types, filenames and Content-Disposition;
- generated least-privilege CSP and platform-owned browser security headers;
- compressed inbound HTTP bodies are rejected unless a future bounded decompression API explicitly supports them.

### Secrets, credentials and cryptography

Implemented:

- `Secret<T>` and `Sensitive<T>` information-flow restrictions;
- explicit public projections; domain/database models are not generically serialized;
- purpose-specific password/token/session/CSRF/key types;
- Argon2id password hashing via platform policy;
- token hash-at-rest, expiry-aware verification, consume-once reset grants, session revocation and rotation proofs;
- purpose/lifecycle-specific signing, verification and encryption key types;
- authenticated user-data encryption using fixed AES-256-GCM, platform-generated nonces and a versioned authenticated envelope;
- active encryption keys can encrypt/decrypt, retiring keys decrypt only, retired keys cannot be used;
- `Redacted<T>` values can only be produced by the trusted `redact(...)` builtin rather than type annotation/casting.

### Integrity and exceptional conditions

Implemented:

- idempotent critical operations bind replay keys to request fingerprints and principal scope;
- webhook replay protection is separate from browser/client idempotency authority;
- first-class closed sum types use exhaustive `match` without wildcard fallthrough;
- transaction outcomes distinguish `Committed`, `RolledBack` and `CommitUnknown`;
- idempotent critical transactions must capture and exhaustively handle transaction outcome;
- public application errors are a closed safe set; internal/database/resource errors remain platform-owned;
- application/runtime failures fail closed rather than selecting permissive fallback behavior.

### Verified pure computation boundary

Implemented:

- pure functions receive no ambient filesystem/network/process/environment/thread/FFI/unsafe authority;
- scalar parameters include by-value `i64`/`bool` plus explicit immutable string/list/struct borrows;
- mutable fixed-`f32` numeric kernels require explicit `&mut` and remain a separate helper ABI;
- scalar pure helpers may call one another with typed expression arguments while inheriting fuel/allocation accounting;
- scalar recursion is hard depth-bounded in generated code; recursive cycles through the mutable numeric hot path are rejected;
- verified `if`/`else if`/`else` evaluates conditions once and preserves static trust/sensitivity metadata;
- `SafeHtml` remains a typed XSS boundary: normal `String` interpolation escapes, while typed `SafeHtml` is not double-escaped;
- domain-specific renderers such as CommonMark remain source-level Velran libraries rather than privileged engine features;
- unknown `@name(...)` template call-directives fail closed instead of silently becoming literal output.

The normative language details are in [`docs/58-verified-pure-functions.md`](docs/58-verified-pure-functions.md).

### Resource and availability safety

Implemented:

- route resource profiles under a platform hard ceiling;
- instruction/allocation budgets and a hard request deadline;
- database list queries require a static bounded row limit and runtime row caps;
- cumulative external-I/O accounting for request fields and outbound traffic;
- outbound response-body hard limits and strict transfer/content-encoding policy;
- file byte/pixel limits and staged processing;
- bounded string/list operations and bounded request collection cardinality.

General unrestricted `Vec<T>` / map collection growth is intentionally not exposed until the same bounded-resource guarantees can be preserved.

### Authentication abuse resistance

Implemented:

- source+principal login attempt limits;
- per-principal limits across sources;
- per-source limits across principals;
- stricter MFA/recovery stage limits;
- fail-closed shared-limiter-store behavior;
- generic credential failure behavior to reduce account enumeration;
- successful login does not erase source-wide/principal-wide abuse history.

### Security monitoring

Implemented:

- typed application security events and mandatory audit for critical operations;
- platform security events for authentication abuse, MFA failures, policy denials, CSRF/origin/CORS failures, webhook verification, idempotency conflicts and resource exhaustion;
- security-event schema does not accept arbitrary request payload/message fields;
- principal/source values are redacted in emitted security events;
- bounded in-memory correlation can use raw keys without writing them to logs;
- platform-owned burst thresholds and alert cooldowns prevent both silent attacks and alert floods.

### Deployment and supply chain

Implemented:

- typechecked `production { ... }` security requirements;
- production startup rejects insecure HTTPS/cookie/development-reload/origin/database-TLS combinations while permitting transactional `reload.mode = "rolling"`;
- source reload cannot silently change the deployment security contract;
- rolling source reload is last-known-good and transactional for additions, edits, renames, and deletions: only a complete valid candidate can withdraw live routes/modules;
- exact `Cargo.lock` plus explicit direct-dependency capability inventory;
- checksummed crates.io-only external provenance in the current policy;
- `VELRAN-SUPPLY-CHAIN.lock` seals Cargo lock + capability policy state;
- release/verification scripts require Cargo lock/manifests consistency before sealing or packaging;
- release manifest hashes the exact source tree.

## High-level OWASP Top 10:2025 posture

This is risk coverage, not a compliance claim.

| Area | Velran status |
| --- | --- |
| Broken Access Control | Strong compiler/runtime enforcement: explicit route access, authorization/mutation proofs, permissions/MFA, tenant isolation, capabilities |
| Security Misconfiguration | Strong production policy, CSP/security headers, secure sessions, egress and startup fail-closed checks |
| Software Supply Chain Failures | Strong foundation: exact lock, capability inventory, provenance restrictions and sealed supply-chain state |
| Cryptographic Failures | Strong: typed credential/token/key purposes, Argon2id, hash-at-rest, lifecycle-aware keys and AEAD user-data encryption |
| Injection | Very strong: SQL/HTML/redirect/header/filename/outbound URL authority are typed or structurally constrained |
| Insecure Design | Strong: critical-operation contracts, idempotency, explicit exceptional states, capabilities and resource invariants |
| Authentication Failures | Strong: platform sessions, rotation/revocation, MFA, typed credentials and multi-dimensional abuse protection |
| Software/Data Integrity Failures | Strong: webhook verification/replay protection, idempotency, AEAD and supply-chain provenance |
| Logging and Alerting Failures | Strong core: typed events, mandatory critical audit, redaction, platform auto-events and burst alerts |
| Mishandling of Exceptional Conditions | Strong: closed public errors, exhaustive sum types, transaction outcome states, deadlines and bounded resources |

## Deliberate remaining non-core work

The seven concentrated hardening iterations are complete. Remaining work should be justified by a concrete threat model rather than by control-count or checklist coverage.

Potential later work:

- controlled `unsafe` boundary with production deny policy, once a real secure-surface exception is needed;
- `velran security --owasp/--asvs/--nist/--iso/--sarif` assurance reporting from compiler/runtime metadata;
- IDE proof/effect visibility and quick fixes;
- generated security architecture documentation;
- generic collections only together with preserved cardinality/resource invariants;
- job/queue payload safety only if a first-class job/queue subsystem is introduced.

## Current verification status

The implemented security model described here now has repository-level execution evidence on the current tree. On the trusted development machine, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving completed successfully. The current status is therefore **Verified Development Milestone**, not merely source/design complete.

This does not collapse repository verification into production acceptance. Target-environment deployment, recovery, secrets/TLS/reverse-proxy configuration, operator acceptance, and any additional environment-specific security/performance evidence remain governed by [`RELEASE-CHECKLIST.md`](RELEASE-CHECKLIST.md).

## Merge and release evidence

A release must not be described as green unless the exact release tree passes the real toolchain gates:

```bash
cargo fmt --all -- --check
cargo metadata --locked --format-version 1 >/dev/null
cargo check --workspace --locked
cargo test --workspace --locked
./verify.sh
```

Repository architecture/clean-code gates are valuable but are not substitutes for Cargo compilation/tests.

## Native-only execution architecture

Application execution is verified IR -> generated safe Rust -> `rustc` -> immutable `cdylib` -> atomic generation activation. The legacy VM, bytecode interpreter and request interpreter are not part of the active execution architecture. There is no semantic or security fallback engine.

Production activation is transactional (`prepare -> verify/load -> activate -> commit`). Native artifacts are content-addressed and integrity/ABI checked. Runtime ABI accounting carries conservative fuel/allocation limits and typed bounded request inputs. A construct without a verified native lowering or typed host ABI fails closed during candidate build or activation; the last known-good generation remains active. Security verification failure is never converted into a permissive execution path.

The native backend covers the Rust-first scalar/numeric/string surface, bounded collections used by the verified subset, typed request inputs, nominal/domain inputs, typed host ABI boundaries, and the supported web execution paths tracked in [`tests/NATIVE_STATUS.md`](tests/NATIVE_STATUS.md). Unsupported semantic families remain explicit pending coverage and are rejected rather than routed to an interpreter.

### Request input and nominal-type boundary

Verified request-bound handler inputs preserve their declared types and bounds across executable IR and the native ABI. Host-owned input slices are bounded; the compiler-owned ABI shim validates tags/reserved fields, per-field byte limits and UTF-8 before constructing safe typed values. The executable-IR verifier rechecks route/handler order and type agreement, and generated safe Rust rechecks the expected input type while charging copied String input against the allocation budget.

`Email`, `Url`, `Slug` and supported nominal domain scalar types keep their domain identity and supported constraints in VerifiedExecutableIR and native cache identity. Generated safe Rust revalidates supported nominal constraints before use. Unsupported domain constraints fail closed at compile/activation time.
