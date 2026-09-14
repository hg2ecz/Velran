<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Maintainability and clean-code boundaries

Velran keeps compiler, runtime, and server responsibilities behind explicit module boundaries. The repository's structural guard is intentionally stricter than the Rust compiler: a build can be valid Rust and still fail the clean-structure check if a façade starts accumulating unrelated responsibilities again.

## Typed configuration errors

Runtime resource-profile construction returns `ResourceProfileError` rather than `String`. Callers can match the error variant while command-line/server boundaries may still render the `Display` message. New configuration APIs should prefer typed errors at library boundaries and convert to human-readable text only at application boundaries.

## Builtin metadata

`BuiltinFunction` owns the stable metadata for every language builtin:

- source name;
- minimum and maximum arity;
- instruction cost;
- request-state dependency;
- execution kind (`Simple` or `Regex`).

The compiler uses the same metadata for generic arity validation, while the runtime uses it for budget charging and execution dispatch. Do not add a parallel builtin-name/cost/regex list in another module.

## Test boundaries

The runtime crate root remains a small façade. Tests that need crate-private helpers import them through `test_support` instead of adding test-only wildcard imports to `lib.rs`. Small boundary modules use explicit imports instead of `use super::*`; this keeps dependencies visible during review and makes future extraction safer.

## Structural guard

Run:

```sh
./tools/check-clean-structure.sh
./tools/check-positive-examples.sh
```

A trusted Rust development environment should additionally run:

```sh
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Refactoring should preserve external Velran semantics. If a module exceeds its guard, extract a cohesive responsibility rather than raising the limit without architectural justification.

## Route declaration scanning boundary

The compiler separates source-level discovery of complete top-level `route` declarations from route semantic parsing. `routes/route_scanner.rs` owns multiline declaration collection, continuation detection, and brace-depth tracking; `routes.rs` owns token interpretation and route validation. Keep this boundary mechanical: scanner changes should determine *which complete declaration text* is handed to the parser, not reinterpret route semantics.

## Web-security policy boundary

The server entrypoint delegates stateless HTTP trust policy to `web_security.rs`. That module owns trusted-proxy interpretation (`Forwarded`, `X-Forwarded-For`, `X-Real-IP`, and `X-Forwarded-Proto`), browser state-change Origin/Referer checks, CORS origin syntax checks, route-scoped CORS preflight, and response CORS headers. Keep authentication/session state and request execution outside this module; it should remain a policy boundary over request metadata and configured web-security settings.

## Schema declaration boundary

The compiler root orchestrates compilation phases but does not own enum/model/form parsing details. `schema_declarations.rs` owns schema-like top-level declarations and form validation-rule semantics; the root retains shared type resolution because queries, routes, templates, handlers, and schemas all consume it. Keep compilation order in the façade and declaration interpretation in the focused module.

## Authentication session boundary

The `auth` crate façade exposes authentication capabilities but does not own session persistence details. `session.rs` owns in-memory and Redis-backed session lifecycle, CSRF comparison, flash-message persistence, authenticated-session rotation, and the `SessionBackend` abstraction. `lib.rs` re-exports the established public session types so callers do not depend on the internal module layout. Keep TOTP, rate limiting, LDAP, and local-user authentication outside the session module; those are separate authentication responsibilities.

## Local-user authentication boundary

The `auth` crate façade does not own local-user persistence or credential policy. `local_user.rs` owns SQLite-backed local users, role persistence, password hashing and verification, TOTP enrollment state, recovery-code consumption, and authentication-generation checks. `lib.rs` re-exports `LocalUserStore` and `LocalUserAuth` so callers remain independent of the internal module layout. Keep LDAP, login rate limiting, generic TOTP verification/replay protection, and session lifecycle outside this module; those remain separate authentication responsibilities.

## Clean-code refactor R18

- Extracted SQLite-backed local-user storage, password policy/hashing, role mutation, TOTP enrollment state, recovery codes, and authentication-generation checks from `crates/auth/src/lib.rs` into the cohesive `local_user.rs` module.
- Reduced the auth façade from 886 lines to 330 lines while preserving the established `LocalUserStore` and `LocalUserAuth` public API through explicit re-exports.
- Kept local-user dependencies explicit and added structural guards so credential/persistence policy cannot silently accumulate back in the auth façade.

## Architecture safety net

The repository has two complementary static architecture gates:

```sh
./tools/check-architecture.sh
./tools/check-clean-structure.sh
```

`check-architecture.sh` protects long-lived design rules rather than individual refactor mechanics. It currently enforces:

- explicit allow-lists for internal crate dependency direction;
- typed errors below application aggregation boundaries;
- bounded façade/entrypoint growth;
- freeze budgets for known one-file hotspots until they are decomposed by responsibility;
- stable explicit re-exports for extracted authentication domains.

Dynamic error erasure is no longer permitted in production Rust code. Application orchestration boundaries use typed aggregation errors, while ordinary non-error trait objects remain valid where runtime polymorphism is intentional.

### Target module map

The next refactor phases should converge toward these responsibility boundaries without forcing directory structure where a single cohesive module is sufficient:

- **server:** process entrypoint, startup/configuration, transport, request dispatch, response/presentation, authentication HTTP flow, security policy, cache and observability bridge;
- **auth:** façade, session, local user, TOTP, rate limiting and LDAP;
- **data:** database execution/transactions, rows, binds/prepared SQL, Redis and transport security;
- **language-core:** values, HTTP/domain wrappers, schema, program, route/AST, configuration and application errors;
- **migrations:** loading, history, locking, SQL splitting and execution;
- **integrations:** egress policy, secrets and HTTPS transport;
- **storage:** filesystem/path policy, multipart upload and image inspection;
- **observability:** request IDs, metrics, logging and log-file lifecycle.

Dependency direction is intentionally conservative. Foundational infrastructure crates must not acquire dependencies on compiler/runtime/server layers merely for convenience. Cross-layer behavior belongs behind data passed downward or an abstraction owned by the consuming layer.

### Refactor acceptance checklist

Every Clean Code refactor should satisfy all of the following before packaging:

1. State one dominant responsibility/boundary being changed.
2. Move behavior before improving behavior; avoid feature work in the same change.
3. Preserve the public API through explicit re-exports unless an API change is intentional and documented.
4. Keep dependencies explicit; do not hide coupling with wildcard imports or convenience globals.
5. Use typed failures at new fallible leaf boundaries.
6. Move or add focused tests with the responsibility when practical.
7. Add a structural guard only for a durable architectural invariant, not an incidental implementation detail.
8. Run architecture, clean-structure, examples, workspace build/test/clippy gates in a Rust-capable environment.
9. Update release notes and the release manifest from the exact packaged tree.

Line budgets are alarms, not design goals. Raising a budget requires an architectural explanation; splitting a cohesive module solely to satisfy a number is also a failure of the intent of the guard.

## Clean-code refactor R19

- Added a repository-level architecture gate separate from historical extraction checks.
- Defined explicit internal crate dependency allow-lists to make dependency direction reviewable and mechanically protected.
- Prevented new boxed/stringly error APIs outside the known application aggregation boundaries.
- Added façade and hotspot growth budgets as regression alarms while preserving responsibility-first refactoring.
- Recorded the target module map and a repeatable refactor acceptance checklist for the remaining R20+ work.

## HTTP presentation boundary

The server entrypoint does not own generic HTTP response rendering. `presentation.rs` owns application/read-error mapping, endpoint error representation, conflict responses, `AppResponse` content negotiation, and validation-form HTML rendering. Request routing, authentication flow, authorization policy, session mutation, and transport I/O stay outside this module. The boundary is intentionally about turning already-decided outcomes into HTTP representations; it must not become a miscellaneous request helper module.

## Clean-code refactor R20

- Extracted generic HTTP response/presentation behavior from `crates/server/src/main.rs` into `presentation.rs`.
- Moved `AppError` and `HttpReadError` response mapping, endpoint JSON/text errors, conflict rendering, `AppResponse` negotiation, and validation-form HTML generation behind one explicit presentation boundary.
- Reduced the server entrypoint from 1537 lines to 1185 lines without changing request routing, authentication/session behavior, route authorization, or response wire semantics.
- Tightened the server-entrypoint architecture budget to 1250 lines and added structural guards preventing presentation responsibilities from growing back into `main.rs`.

## Authentication HTTP boundary

The server entrypoint does not own login/logout transport behavior. `auth_http.rs` adapts HTTP requests to the authentication/session domain: it owns the reserved login and logout endpoint handlers, form decoding and CSRF checks for those endpoints, authentication activity audit events, authenticated-session rotation/invalidation, and session-cookie wire formatting/parsing. The `auth` crate remains transport-independent, while generic response presentation stays in `presentation.rs`. Route authorization and general request dispatch remain outside this module.

## Clean-code refactor R21

- Extracted authentication HTTP flow and session-cookie transport from `crates/server/src/main.rs` into `auth_http.rs`.
- Reduced the server entrypoint from 1185 lines to 869 lines while preserving login/logout paths, CSRF behavior, rate limiting, LDAP/local authentication, TOTP/recovery-code handling, session rotation, and cookie semantics.
- Kept module dependencies explicit and added structural guards preventing auth HTTP/session-cookie concerns from returning to `main.rs`.
- Tightened the server-entrypoint architecture budget to 950 lines.

## Lifecycle and operations boundary

The server entrypoint does not own process lifecycle helpers or operational listeners. `operations.rs` owns panic-hook installation, shutdown signal handling, liveness/readiness responses, the optional HTTP-to-HTTPS redirect listener, redirect-target safety validation, and the dedicated metrics listener. Application request dispatch and domain behavior remain outside this module. The boundary is intentionally operational: it must not become a home for general request helpers.

## Clean-code refactor R22

- Extracted lifecycle and operational endpoint behavior from `crates/server/src/main.rs` into `operations.rs`.
- Reduced the server entrypoint from 869 lines to 660 lines while preserving health, shutdown, redirect, panic logging, and metrics behavior.
- Kept module dependencies explicit and added structural guards preventing operational concerns from returning to `main.rs`.
- Tightened the server-entrypoint architecture budget to 720 lines and added a 280-line budget for `operations.rs`.


## Request input adaptation boundary

The server entrypoint does not own request-body representation details. `request_input.rs` owns strict scalar JSON-object decoding, media-type normalization/comparison, and the transformation of persisted uploads into runtime `Value` instances. Multipart streaming and connection-level byte handling stay in the transport/dispatch layer; route semantics and validation stay outside this adapter. The boundary is intentionally about translating accepted wire input into runtime representations, not about request orchestration.

## Clean-code refactor R23

- Extracted request input adaptation from `crates/server/src/main.rs` into `request_input.rs`.
- Reduced the server entrypoint from 660 lines to 549 lines while preserving JSON duplicate-field rejection, scalar-only JSON policy, media-type matching, and upload/image runtime mapping.
- Kept module dependencies explicit and added structural guards preventing input adaptation from returning to `main.rs`.
- Tightened the server-entrypoint architecture budget to 600 lines and added a 160-line budget for `request_input.rs`.

### R24: typed orchestration errors

The final dynamic error aggregation boundaries were removed from server startup, connection handling, and the command-line application.

`StartupError` and `ConnectionError` live under the server error boundary and aggregate only the concrete failures the orchestration layer is expected to propagate. `CliError` performs the same role for the CLI. Concrete wrapped errors remain available through `Error::source`; operator or usage policy failures are represented explicitly as message variants.

The architecture check now rejects `Box<dyn Error>` in production Rust code. Trait objects used for non-error abstractions such as server I/O remain valid. This distinction prevents dynamic error erasure from returning while preserving ordinary runtime polymorphism where it belongs.

### R25: data responsibility boundary

The `data` crate is now a facade rather than a one-file implementation hotspot. Its responsibilities are separated into `types`, `sql`, `database`, `redis_store`, and `error`. The dependency direction is intentional: shared types and errors are leaf concepts; SQL preparation depends on those shared types; database execution depends on prepared SQL; Redis remains independent of SQL/database execution. External callers continue using the stable `data::{...}` facade. This keeps transport policy, SQL safety, execution, and Redis behavior independently reviewable without creating a new public API surface.


### R26: storage responsibility boundary

The `storage` crate is now a thin facade over three responsibility-focused modules. `filesystem` owns confined path validation, openat2-based file access, bounded streaming writes, and staged filesystem primitives. `upload` owns multipart policy, CSRF/file cardinality rules, filename validation, and staged upload commit/cleanup orchestration. `image` owns byte-level PNG/JPEG inspection and pixel-limit enforcement. External callers continue using the stable `storage::{...}` facade, while upload code receives only narrow crate-internal filesystem capabilities rather than direct access to filesystem representation fields.


### R27: integrations responsibility boundary

The `integrations` crate is now a thin facade over four focused modules. `egress` owns target configuration, hostname/CIDR validation, DNS-address policy helpers, and immutable target limits. `secrets` owns confined secret-file access and zeroizing secret values. `https_client` owns DNS resolution, connection/TLS establishment, outbound HTTP framing, and bounded response parsing. `error` owns the shared `IntegrationError` boundary. The HTTPS client receives only crate-internal policy and secret capabilities; the external `integrations::{...}` facade remains stable.

### R28: observability responsibility boundary

The `observability` crate is now a thin facade over four focused modules. `events` owns structured log schemas, UTC timestamps, request IDs, and serialization. `metrics` owns counters, latency histograms, bounded route metrics, request timing, and the log-fallback metric. `logging` owns file sinks, the bounded asynchronous writer queue, rotation reopen/flush lifecycle, and the process-global logging bridge. `error` owns the typed serialization failure. Logging increments the metrics-owned fallback counter through a narrow crate-internal capability, avoiding a cyclic dependency while keeping the metric next to the metrics exporter. External callers continue using the stable `observability::{...}` facade.

### R29: migrations responsibility boundary

The `data::migrations` module is now a thin facade over lifecycle-focused modules. `source` owns safe migration discovery, filename/version validation, hashing, and SQL splitting. `history` owns the invariant that applied migrations are immutable and that pending versions cannot be backfilled behind already-applied history. `database` owns connection/transport policy, migration-state persistence, and raw statement execution. `locking` owns backend-specific migration lock acquisition and release. `service` composes these capabilities into `status`, `verify`, and `apply`. Shared migration records and the typed `MigrationError` remain leaf concepts in `types` and `error`. External callers continue using the stable `migrations::{...}` facade, while lock/database/source details remain crate-internal.

### R30: language-core foundation boundary

The first `language-core` split moves only foundational types, deliberately leaving the AST/program layer for later steps. `web_types` owns HTTP/presentation value objects: method parsing, trusted compiler HTML, redirects, and flash messages. `values` owns runtime/domain scalar and aggregate value representation, image references, value type metadata, and function parameter types. The crate root re-exports these types unchanged so compiler, runtime, and server callers keep the stable `language_core::{...}` API. This staged split avoids mixing AST reorganization with foundational-value movement and gives R31-R32 smaller, reviewable dependency changes.

### R31: language-core AST boundary

The second `language-core` split moves the execution/compile-time syntax model out of the crate root. `ast` owns builtin metadata, expressions, HTML templates, query calls, authorization metadata, compute/page/action statements, transaction statements, resource/source locations, and page/action function bodies. It depends only on the foundational `values` and `web_types` modules. The crate root continues to re-export every moved type unchanged, so compiler/runtime/server callers retain the stable `language_core::{...}` surface. Schema, query, route, program, server configuration, and application errors remain in the root for the final R32 split.

### R32: language-core program/schema boundary

The third `language-core` split completes the staged decomposition. `schema` owns enums, models, form schemas, form failures, and validation rules. `query` owns query capability, return contracts, and query functions. `routing` owns route segments, upload metadata, route authentication, public-cache policy, and route declarations. `program` owns the aggregate compiled program and lookup helpers and depends on the AST plus schema/query/routing contracts. `config` owns server defaults, while `error` owns `AppError`. The crate root is now a thin facade that preserves the existing `language_core::{...}` API. This establishes an explicit dependency ladder from foundational values/web types through schema/query/routing and AST into the aggregate program without forcing downstream crates to know the internal module layout.

### R33: compiler expression boundary

The second compiler/runtime audit starts with the compiler root's largest mixed responsibility. `expression` owns expression tokens, precedence parsing, expression validation, and static type inference. The lexer now names the expression module as the token owner instead of relying on the compiler root as an implicit dependency hub. A temporary crate-internal re-export keeps existing compiler modules and tests source-compatible during the staged R33-R36 audit; it is not part of the public compiler API and can be narrowed as later steps remove root/wildcard coupling. Generic source scanning, SQL bind scanning, declaration parsing, and compile orchestration remain outside this module so R33 has one dominant architectural goal.

### R34 compiler utility boundaries

R34 removes the compiler root's remaining syntax-utility hub role. Generic source scanning (identifier parsing, balanced delimiters, top-level splitting, statement-end scanning, whitespace/comment skipping) now belongs to `source_syntax.rs`, while SQL keyword and named-bind scanning belongs to `sql_syntax.rs`. The root retains temporary crate-internal re-exports only as a staged migration seam; ownership is explicit and future audit steps can migrate callers directly to the owning modules.

### R35: compiler explicit dependency boundary

The compiler responsibility split is now reflected in import direction, not only file placement. Production parser and validation modules import expression, source syntax, SQL syntax, domain-symbol and type-resolution helpers directly from their owning modules instead of relying on `use super::*` or crate-root compatibility re-exports.

The crate root remains compilation orchestration and public facade. Test-only compatibility imports are intentionally retained until the dedicated test-architecture phase (R37-R39), where the large legacy test module will be decomposed without forcing production modules back into implicit coupling.

### R36: runtime statement execution boundary

The runtime request layer now owns request matching, input binding, resource-profile orchestration and panic containment only. Page/action statement interpretation, object authorization, transaction/business-audit execution and JSON response-value serialization live in `statement_execution.rs`. `AppResponse` is owned by a small neutral `response.rs` module, avoiding a dependency cycle between orchestration and interpretation.

Regression guards keep `request_execution.rs` below 260 lines, require explicit statement delegation, and prevent statement/audit/serialization implementation from moving back into the request orchestrator.

### R36.2 explicit compiler import regression fix

R35 intentionally removed crate-root wildcard coupling from compiler production modules. R36.2 completes that transition by making every remaining collaborator import explicit at the module that uses it. This is a build-fix rather than a new responsibility split: behavior is unchanged, but the dependency graph is now mechanically visible and guarded.

### R37: compiler test architecture

Compiler tests now have their own module boundary instead of borrowing a large `#[cfg(test)]` prelude from the compiler crate root. `tests.rs` is a small test facade and the compile tests are grouped by responsibility (`core`, `presentation`, `domain`, `data contract`, and `web flow`), alongside the existing focused numeric/builtin test modules. This keeps production dependency wiring independent from test convenience imports and gives later test cleanup a stable ownership map.

### R38: runtime test architecture

Runtime tests now mirror the production responsibility map instead of living in a single 1,400+ line file. `tests.rs` is a small facade and feature tests are grouped into `core`, `database`, `serialization`, `presentation`, and `domain` modules. The existing `test_support` module remains the shared test capability boundary, while the production runtime root does not gain test-only convenience imports. This keeps test navigation and ownership aligned with runtime architecture without changing production behavior.

### R39: server test architecture

The server test suite now mirrors production responsibilities instead of accumulating in one `main_tests.rs` file. The facade only declares four responsibility modules: HTTP/security, configuration/lifecycle, rate-limit/observability, and response/typed-boundary tests. Existing nested test modules remain intact inside those files, keeping the change mechanical while making ownership and future growth explicit.

### R40: workspace crate-boundary review

The final audit applies a stricter rule than "small code belongs in a crate": modules are the default, and a crate is justified only when the compiler-enforced dependency boundary provides real value (independent API/reuse, distinct dependency profile, security/runtime boundary, or separate executable lifecycle).

The review consolidated `resource-limits` into `velran-server::resource_limits`. The former crate was small, had exactly one consumer, and only configured the server process at startup. Keeping it as a crate created Cargo/workspace/API wiring without an independent component boundary. Typed errors, fail-closed resource-limit validation, Linux RLIMIT/cgroup behavior, and tests remain intact inside the server module.

The `integrations` crate was reviewed but retained. It is not currently consumed by another workspace crate, yet it is explicitly documented as a trusted outbound adapter and carries its own TLS, egress-policy, DNS/IP, and secrets responsibilities. That is a meaningful dependency/security boundary rather than an accidental micro-crate.

After R40 the workspace has 11 crates. Future additions should default to modules unless they meet the same boundary test.

### R41: server explicit dependency boundary

The server responsibility split is now reflected in production import direction. Child modules no longer use `use super::*` to inherit the server root's large import surface. Connection handling names request-pipeline, dispatch, presentation, HTTP I/O, TLS, and security collaborators directly; startup names backend, auth, lifecycle, rate-limit, and reload collaborators directly; configuration/static/TLS modules likewise import only their owning capabilities. The `main_tests` wildcard prelude remains intentionally test-only and is outside this production dependency rule.

A regression guard rejects `use super::*` in server production child modules. This keeps the server root from becoming an implicit service locator as responsibilities evolve.

### Server crate-root facade rule
The server binary root is an orchestration boundary, not a child-module prelude. Production code must import only the specific child-module items used by `main.rs`; responsibility tests own their shared helper imports in `main_tests.rs`. Wildcard child-module imports in the server root are rejected by the clean-structure checks.

### R44: server CLI responsibility boundary

The server CLI entry module is now orchestration only. `cli_scan` owns the early bootstrap scan for `--config`, `--check-config`, and `--print-effective-config`; `cli_overrides` owns command-line override application over the already-loaded configuration; and `cli_finalize` owns cross-field validation, domain construction, `--check-config`, and `--print-effective-config` behavior. `cli.rs` only composes those stages.

R44 intentionally preserved the existing positional `ParsedArgs` contract while splitting CLI responsibilities, keeping that refactor narrow. R45 then completed the planned second step by replacing the long tuple with the named `StartupArgs` boundary. Keeping those two changes separate made both ownership changes reviewable and prevented parser decomposition from being coupled to startup data-model migration.


### R45: named startup argument boundary

The CLI-to-startup handoff is now a named `StartupArgs` structure rather than a positional 20-field tuple. This makes ownership and review explicit: logging and process resource limits are accessed by field name in `main`, while `startup` destructures the remaining runtime configuration by responsibility. Positional access such as `parsed.15` is rejected by structural checks. The structure is owned by `startup_args.rs`, keeping the server binary root focused on orchestration rather than data-model declarations.

### R46: startup service-preparation boundary

Server startup now separates service construction from listener/process orchestration. `startup_services.rs` owns domain collection, database/Redis connection checks, route-rate-limit setup, public-cache setup, session backend selection, authentication runtime construction, protected-route validation, and source-reload supervisor preparation. `startup.rs` remains responsible for startup order, transport/TLS policy, listener lifecycle, signal handling, connection spawning, and graceful shutdown. The boundary uses named `ServicePreparation` and `PreparedServices` structures so the split does not introduce positional parameter bundles or hidden root dependencies.

### R47: startup transport lifecycle boundary

Server startup now separates validation/service preparation from transport execution. `startup.rs` owns startup argument destructuring, hosting/runtime construction, service preparation, and TLS/reverse-proxy policy validation. `startup_transport.rs` owns application listener binding, metrics/redirect listener tasks, SIGHUP log/reload handling, connection admission/spawning, TLS handshake dispatch, shutdown-signal handling, Unix-socket cleanup, task cancellation, and graceful connection draining.

The handoff uses a named `TransportRuntime` structure instead of a positional tuple or long parameter list. Structural guards keep listener/JoinSet/shutdown logic out of `startup.rs` and require transport ownership to remain explicit.


## R48 module namespace boundary

Velran module loading and symbol identity are deliberately separate concerns. `mod child;` adds the declaring module's child source unit to the application graph, while declarations remain owned by the child namespace. `crate::`, `self::`, and `super::` are explicit path anchors. Cross-module references remain canonical `a::b::Symbol` paths; `use path as alias;` may shorten only an explicit namespace prefix. Filesystem traversal stays forbidden. This keeps dependencies visible and deterministic without building a second general Rust scope/type checker.

### R49: example suite as a compatibility contract

The user-facing examples under `examples/` are enumerated by `tests/manifests/example-entrypoints.txt`. `tools/check-positive-examples.sh` compiles every listed application entrypoint and also rejects any Velran example directory that is missing from the manifest. Negative and security rejection fixtures live under `tests/fixtures/`, and verification-only manifests live under `tests/manifests/`. This keeps educational material separate from test data while still making the examples an explicit language-compatibility surface. The `module-namespaces` and `module-imports` examples exercise canonical file mapping, module-relative child declarations, nested `pub mod` visibility, and explicit namespace aliases.
## R50 documentation/example namespace synchronization

The namespace documentation now follows the iteration-6 contract: child `mod` paths are declaring-module-relative; `crate::`, `self::`, and `super::` are explicit anchors; `pub mod` controls nested module visibility; and `use path as alias;` aliases only a namespace prefix. Wildcard/bare-symbol imports remain fail-closed. The package-ownership layer adds only package-root `pub use path as alias;` for already-public module namespaces; direct item, private-module, and private transitive-dependency re-exports remain rejected. Example-file trees in prose must stay consistent with the canonical file mapping.


## Velran module/import boundary

The Velran compiler keeps module-header parsing in `module_header.rs`; source loading owns filesystem confinement and dependency traversal; ordinary declaration parsers do not each reimplement `mod`/`use` handling. Import aliases are explicit namespace-prefix aliases rather than a second general Rust scope resolver. This preserves one responsibility per layer and avoids duplicating `rustc` name/type-checking behavior.

Module visibility and item visibility are separate contracts. `pub mod` opens a namespace boundary; item-level `pub` opens an ordinary library API. Neither changes route/capability authority. Only the narrow package-root `pub use path as alias;` form may re-export an already-public module namespace; wildcard, direct-item, private-module, and private-transitive re-exports remain fail-closed.

## Package boundaries
Use local libraries as architecture boundaries: expose a small `pub` API, keep implementation modules private, and keep application authority out of reusable computation packages. Prefer `app -> small public library API -> private implementation` over cross-directory coupling.
