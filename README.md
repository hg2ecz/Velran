<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Velran — a security-first web application language and server with Rust syntax.

Velran is a Rust-based language/runtime/server ecosystem specialized for web application development. The V1 scope focuses on secure-by-default behavior, typed input/output, compiler-enforced policies, explicit capabilities, auditability, and production operability.

## Getting started

For the canonical English documentation, start here:

1. [Documentation index](docs/README.md)
2. [Dependency security and reproducible builds](docs/19-dependency-security.md)
3. [Optimistic locking and concurrent edits](docs/24-optimistic-locking.md)
4. [V1 release notes](RELEASE-NOTES-V1.0.md)

The production-oriented reference application is in `examples/starter-project/`.

Verification-only rejection fixtures and manifests live under `tests/`; user-facing examples remain under `examples/` even when `verify.sh` also compiles them.

Canonical documentation is English. Hungarian documentation is kept separately under [`docs/hu/`](docs/hu/) and the Hungarian project overview is [`README_hu.md`](README_hu.md). The canonical English book source is under [`docs/book/`](docs/book/), with the Hungarian translation under [`docs/book/hu/`](docs/book/hu/).

## Verified development milestone

The current Velran implementation has been consolidated into this verified baseline. The design target remains security first, then developer productivity and runtime/compiler performance, with clean-code boundaries kept explicit.

This tree has now completed the repository-level development gate on a real Rust toolchain: `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving are all reported successful on the current source tree. This is the **Verified Development Milestone** baseline used by the documentation and books.

This milestone is not a substitute for target-environment production evidence. Deployment topology, recovery drills, secrets/TLS/reverse-proxy configuration, operator acceptance, and any environment-specific performance or security evidence remain release responsibilities. See [`RELEASE-CHECKLIST.md`](RELEASE-CHECKLIST.md) and [`ITERATION-1-NOTES.md`](ITERATION-1-NOTES.md).

## Build and verification

The workspace uses Rust edition 2024. The repository intentionally does not pin a specific compiler version; reproducible dependency resolution is provided by the committed `Cargo.lock` and `--locked` builds.

**Runtime requirement: `rustc` is required on every host that runs Velran applications.** Velran's only application execution backend is the native Rust backend: verified Velran code is lowered to safe Rust and compiled by `rustc` into an immutable `cdylib` before activation. Installing `velran-server` alone without an available Rust compiler is therefore not sufficient. `rustc` may be discovered from the active toolchain or configured explicitly with an absolute path in the `[native]` section of `server.toml`. Cargo is required to build Velran itself from source, but application activation uses `rustc` as the backend compiler.

```bash
./verify.sh
cargo build --locked --release -p velran-server -p velran-cli
```

Installed binaries are intended to live at:

```text
/usr/local/bin/velran-server
/usr/local/bin/velran-cli
```

The primary production configuration path is:

```text
/usr/local/etc/velran/server.toml
```

Start the server with:

```bash
velran-server --config /usr/local/etc/velran/server.toml
```

Preflight validation:

```bash
velran-server --config /usr/local/etc/velran/server.toml --check-config
```

Configuration precedence is:

```text
defaults < TOML config < targeted CLI override
```

Stable production policy belongs in trusted configuration rather than in a long command line. SIGHUP reopens logs and, in behind-proxy mode, transactionally reloads domain/application hosting state. Application source changes are also detected automatically by the shared source-reload supervisor; process-level settings still require restart. See `docs/39-automatic-source-reload.md` for the deployment and failure semantics.

## Debian package

On Debian/Ubuntu build hosts, `make deb` creates an installable `velran_1.0.0-1_<arch>.deb`. Debian-managed installs intentionally use `/usr/bin` and `/etc/velran` rather than `/usr/local`; see [the Debian packaging guide](docs/36-debian-package.md).

## V1 capability overview

- typed routing, forms, JSON APIs, domain/nominal types and exhaustive enum matching;
- typed SQL bind/decode, static DB row bounds, migrations, optimistic locking and explicit transaction outcomes;
- local/LDAP authentication, TOTP/MFA, permissions, object/mutation authorization and multi-dimensional abuse protection;
- platform-owned sessions plus purpose-safe token issuance, hash-at-rest, expiry, revocation and rotation;
- `Secret<T>` / `Sensitive<T>` flow control, explicit public projections and trusted redaction;
- critical-operation contracts with audit, transaction and idempotency requirements;
- tenant membership authority and compiler-enforced tenant isolation;
- verified/replay-protected webhooks and staged/verified file publication;
- effect/capability tracking and named SSRF-hardened outbound integrations;
- typed HTTP metadata, generated least-privilege CSP/security headers and strict production deployment policy;
- purpose/lifecycle-specific cryptographic keys and AES-256-GCM authenticated user-data encryption;
- request/DB/file/outbound resource ceilings, hard request deadlines and cumulative external-I/O budgets;
- structured security events, redaction and burst-alerting foundation;
- supply-chain capability/provenance lock plus reproducible release evidence;
- config-first deployment plus backup/restore/upgrade/rollback workflows.

## Deliberate V1 non-goals

The V1 core does not include full-text search, background jobs, email sending, a scheduler, revision history, a workflow engine, soft delete, generated admin CRUD, package/registry distribution, general-purpose `pub use` item/wildcard re-export semantics, S3 media, image resize/thumbnail generation, HTTP/2 or HTTP/3, SSE/WebSocket, an OpenTelemetry SDK, or private cache. The only supported re-export is the narrow package-root `pub use path as alias;` form for already-public module namespaces.

Velran now has an intentionally narrow Rust-like item visibility surface for reusable library code: ordinary structs, enums, pure functions, inherent methods and struct fields are private by default and use `pub` for cross-module API exposure. `impl` blocks themselves are never public; individual methods are. Framework callables remain governed by route/capability policy, not `pub`. Child `mod` declarations are relative to their declaring module; `crate::`, `self::`, and `super::` provide explicit anchors. Canonical nested module mapping follows namespace paths, so `foo::bar` maps to `foo/bar.vrn`. Nested library boundaries support `pub mod`, and `use path as alias;` provides explicit namespace-prefix imports. A narrow `pub use path as alias;` form is supported only at the package root and only for already-public module namespaces; wildcard, item, private-module, and private transitive-dependency re-exports remain fail-closed. Compute locals use Rust-like `let mut` plus direct assignment (`value = ...`, `array[index] = ...`); the legacy `set` syntax is rejected. Persistent business-state mutation still goes through explicit query/transaction/action paths.

## Language syntax essentials

Velran uses explicit statement boundaries. Simple statements end with `;`; newlines are whitespace only and there is no automatic semicolon insertion. Non-block top-level declarations such as `mod path;` and `route ... => handler;` also end with `;`, while block declarations and control-flow blocks end with `}` and do not take a trailing semicolon. See [`docs/56-statement-terminators.md`](docs/56-statement-terminators.md).

Routes also require an explicit access decision: use `public` for intentionally public endpoints or `auth user` / `auth mfa` / `auth role ...` for protected endpoints. Dynamic String redirects are rejected; normal redirects use compiler-checked typed route calls. Raw external `String` inputs receive an automatic 4096-character upper bound unless a narrower route or domain-type validation is declared. Reusable domain contracts over Rust-like scalar spellings and `splitBounded(...)` make common validation/resource limits concise. See [`docs/57-secure-by-construction-foundation.md`](docs/57-secure-by-construction-foundation.md), [`docs/62-domain-types-and-bounded-collections.md`](docs/62-domain-types-and-bounded-collections.md), and the canonical [security status](SECURITY-STATUS.md).

The numeric core includes checked `+`, `-`, `*`, `/`, `%`, integer shifts `<<`/`>>`, integer bitwise `&`/`^`/`|`, boolean `!`/`&&`/`||` with short-circuit evaluation, and Rust-like `f32` math methods including `.ln()`, `.log10()`, `.log()`, `.exp()`, `.powf()`, `.round()`, `.floor()`, and `.ceil()`. See [`docs/44-math-and-timing.md`](docs/44-math-and-timing.md).

The Unicode-aware string core prefers Rust-like methods such as `.trim()`, `.trim_start()`, `.trim_end()`, `.to_lowercase()`, `.to_uppercase()`, `.chars().count()`, `.contains()`, `.starts_with()`, `.ends_with()`, `.replace()`, and `.repeat()`, with bounded framework helpers such as `splitBounded(...)` where the secure web contract needs an explicit cardinality limit. See [`docs/46-string-builtins.md`](docs/46-string-builtins.md) and [`docs/49-regular-expressions.md`](docs/49-regular-expressions.md).

## Native-only execution

Execution is verified IR -> generated safe Rust -> `rustc` -> immutable `cdylib` -> atomic generation activation. There is no VM/interpreter fallback. A construct without a verified native lowering or typed host ABI fails closed during candidate build/activation, and the last known-good generation remains active. See [`tests/NATIVE_STATUS.md`](tests/NATIVE_STATUS.md) for the current native coverage matrix.

### Native coverage: nominal request inputs

The verified rustc backend preserves distinct `Email`, `Url`, `Slug`, and supported bounded nominal domain request inputs across the EIR and cache identity instead of erasing them to primitive values. Unsupported constraints are rejected rather than routed to a fallback engine. See [`tests/NATIVE_STATUS.md`](tests/NATIVE_STATUS.md) and the domain-type documentation under [`docs/`](docs/README.md).


### Native rustc backend configuration

For production, the native backend can be configured explicitly:

```toml
[native]
enabled = true
cache_dir = "/var/cache/velran/native"
optimization = "release"
```

`cache_dir` must be an absolute path. `rustc` is optional; omitting it uses the active toolchain discovery path, which is recommended for rustup-managed toolchains. If `rustc` is configured explicitly, it must be an absolute compiler path. Omitting the whole section preserves automatic rustc/cache discovery.


## Documentation language policy

English is the canonical project documentation language. Hungarian material lives under `docs/hu/` or uses an explicit `_hu` suffix. See [the documentation index](docs/README.md) for the current layout. Historical iteration/build-fix notes are separated under [`history/development-notes/`](history/development-notes/) and are not normative documentation.

### Rustc-first diagnostics and security gate

The native backend deliberately uses trusted `rustc` as the Rust syntax/type authority rather than re-implementing it. Velran performs the web sandbox checks first (capabilities, unsafe/ambient authority, resource/security contracts), then invokes isolated `rustc` for the generated cdylib. Native compiler failures retain the real bounded rustc diagnostic for console/error logging; `--debug-compile-errors` can additionally expose an HTML-escaped developer page during non-production source reload while the last valid generation keeps serving.

### Rust-first inherent methods

Velran now accepts a deliberately narrow Rust-like inherent-method surface: `impl Type { fn method(&self, ...) { ... } }` plus authority-free associated functions such as `Type::new(...)`. Inside an inherent impl, `Self` may be used in safe parameter/return positions and struct literals; it is resolved by the compiler to the declaring struct before verified lowering. The compiler does not paste impl bodies into generated Rust. It registers each method and lowers the body through the existing verified pure-compute path, preserving fuel/allocation accounting and the capability boundary. Public methods are checked so private structs cannot leak through public parameter or return types. Owned or mutable `self`, generic impls and trait impls remain fail-closed in the current language contract; they are not implied by completion of the countdown work. Cross-package public structs, constructors and immutable methods use the same visibility and package boundaries. The resulting safe Rust is still typechecked and optimized by the real `rustc`.


## License

Velran is licensed under the Mozilla Public License 2.0 (`MPL-2.0`). Copyright (c) 2026 Zsolt Krüpl. See [`LICENSE`](LICENSE) and [`COPYRIGHT`](COPYRIGHT). Third-party dependencies remain subject to their own licenses.
