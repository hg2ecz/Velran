<!-- VELRAN-DOC-STATUS: 2026-09-15 -->
> **Documentation status (2026-09-15):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Automatic application source reload

Velran can activate newly deployed `.vrn` application code without a process restart. Source reload is transactional: a changed application is compiled and validated as a candidate, and the live domain runtime is replaced only after the candidate succeeds. If the new code is invalid, the previous generation keeps serving traffic.

## Configuration

Global defaults:

```toml
[reload]
enabled = true
mode = "development"
poll_interval_ms = 1000
debounce_ms = 250
```

Per-domain overrides:

```toml
[[domains]]
host = "example.com"
workdir = "/srv/velran/domains/example.com/current"
app = "main.vrn"

[domains.reload]
enabled = true
mode = "development"
poll_interval_ms = 1500
debounce_ms = 300
```

Automatic source reload can be disabled for maintenance with `reload.enabled = false`, with `domains.reload.enabled = false` for one domain, or process-wide with `--no-source-reload`.

## What is watched

The reload supervisor watches the Velran source roots and fingerprints relevant `.vrn` and `velran.toml` files with metadata plus SHA-256 content hashes. Fingerprinting happens on the polling/debounce path, never on ordinary HTTP requests. This detects same-size edits and supports safe candidate validation for direct-upload workflows.

The configured logical `app` path is watched in addition to canonical module paths. This matters for atomic release layouts such as:

```text
/srv/velran/domains/example.com/current -> releases/2026-09-05.2
```

Switching the `current` symlink therefore triggers a new compile even when the old dependency graph points into the previous canonical release directory.

There is one shared supervisor for all domains rather than one polling thread per domain. Each domain is checked according to its effective `poll_interval_ms`.

## Change, debounce, compile, commit

When the source fingerprint changes, Velran waits for the source set to remain stable for `debounce_ms`. This avoids recompiling once per file while a release is being copied or uploaded.

The candidate application is then compiled and validated using the same hosting checks used during startup/reload, including route/resource/cache/auth constraints. Compilation runs away from request handling. Before activation Velran fingerprints the live source tree again; if any source changed while the candidate was building, that candidate is discarded and a new debounce/build cycle starts. Only a stable candidate can atomically replace the domain runtime. Requests already holding the previous runtime finish on the old generation; later requests use the new generation.

A successful source-code commit also advances the domain's public-cache generations for cached routes. Old generated HTML/JSON is therefore not kept visible merely because its previous TTL has not expired.

## Modules added, removed, or uploaded late

Adding a module is detected through its already-known parent source: adding a new `mod` declaration changes the parent file and triggers compilation. After a successful compile, the newly discovered module becomes part of the watched dependency graph automatically.

Removing or renaming a watched module is also a change. If the module and all references/routes that depend on it are removed consistently, the candidate can compile successfully and the removed module, handler, and route disappear from the newly activated generation. If any live source still references the missing module or symbol, the candidate fails validation, the error is logged, and the old application generation remains active. Deletion is therefore transactional: it either becomes part of a complete valid generation or has no effect on live traffic.

A multi-file upload may temporarily contain a parent module that references a new child file which has not arrived yet. In that case the first candidate compile can fail legitimately. Velran retries a stable failed candidate with exponential backoff, starting at 2 seconds and capped at 60 seconds. This lets a late module activate without continuously recompiling permanently broken code.

## Failure behavior and logging

A failed automatic reload does **not** take the domain offline. The previous valid generation remains active. Structured events include:

- `source_change_detected`
- `reload_candidate_started`
- `reload_candidate_ready`
- `reload_candidate_source_changed`
- `reload_activated`
- `reload_previous_generation_retained`
- `source_reload_rejected`
- `source_reload_cache_invalidation_failed`
- `source_reload_stale`

`source_reload_rejected` includes the canonical domain, active generation, and compiler/validation error so syntax and module errors remain diagnosable from the server log. Frontend expression/control-flow failures preserve source location, including line information through nested pure `if`/`else`/`while` parsing. Native cdylib failures also retain the bounded real `rustc` stderr instead of replacing it with an Velran imitation. With non-production `debug_compile_errors = true` (or `--debug-compile-errors`), the detailed frontend/rustc diagnostic is additionally written as `source_reload_diagnostics` and shown as an HTML-escaped, domain-scoped developer error page for GET/HEAD requests. Health endpoints remain available and the last valid generation is not discarded. Production policy rejects detailed compiler diagnostics.

## Deployment workflow

Velran supports two application-only deployment styles. For rolling production, a web developer may upload or edit the watched `.vrn` files directly; debounce, stable-source fingerprinting, candidate validation, and atomic activation protect the live generation. For controlled releases, immutable release directories plus an atomic `current` symlink remain useful but are optional rather than required.

Recommended rolling sequence:

1. upload the changed `.vrn` files into the watched application tree;
2. let the source-reload supervisor wait for a stable fingerprint and build the candidate;
3. confirm `reload_activated`, health/readiness, and application smoke checks.

For a controlled immutable release, optionally run `velran-server --config ... --check-config`, switch the `current` symlink atomically, then perform the same activation and smoke-check verification.

Changes to process-level settings such as listeners, database/Redis/auth connections, cgroup limits, or logging sinks still require the normal configuration/restart lifecycle. `SIGHUP` in behind-proxy mode is for transactional domain/application configuration reload; automatic source reload is the lighter application-code path.


## Rolling production mode

For a PHP-like workflow where a developer uploads changed source files directly, use:

```toml
[reload]
enabled = true
mode = "rolling"
poll_interval_ms = 1000
debounce_ms = 1000
debug_compile_errors = false
```

`rolling` is the production-safe reload mode. A strict `production { debug disabled; ... }` policy permits it because activation remains transactional and last-known-good. The same policy still rejects `mode = "development"`, detailed compiler-error pages, insecure development cookies, and `native.debug_rustc_repro = true`. Startup with `native.required = true` still fails closed if the initial native runtime cannot be built; a later rolling candidate failure only rejects that candidate and leaves the current generation active. A complete sample is `config/server-rolling-prod.toml.sample`.

## Native runtime configuration

The native `rustc` backend can be configured explicitly in the server TOML. The section is optional; omitting it preserves automatic toolchain discovery and the per-application `.velran-native-cache` default.

```toml
[native]
enabled = true
cache_dir = "/var/cache/velran/native"
optimization = "release"
# For maximum speed on a host-local cache, let rustc tune for this CPU:
# target_cpu = "native"
# Optional explicit feature override/addition:
# cpu_features = "+sse2,+avx2"
```

`cache_dir` must be an absolute path. `rustc` is optional; automatic active-toolchain discovery is recommended for rustup-managed installations. If configured explicitly, `rustc` must be an absolute compiler path. `optimization` is either `interactive` (`-O2`) or `release` (`-O3`). Explicit TOML values take precedence over `RUSTC`, `VELRAN_NATIVE_CACHE`, and `VELRAN_RUSTC_CPU_FEATURES`; environment variables remain compatibility fallbacks when the corresponding TOML field is omitted. Production application execution is native-only: disabling native bootstrap means application activation cannot fall back to a VM/interpreter path.
