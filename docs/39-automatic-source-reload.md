<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Automatic application source reload

Velran can activate newly deployed `.vrn` application code without a process restart. Source reload is transactional: a changed application is compiled and validated as a candidate, and the live domain runtime is replaced only after the candidate succeeds. If the new code is invalid, the previous generation keeps serving traffic.

## Configuration

Global defaults:

```toml
[reload]
enabled = true
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
poll_interval_ms = 1500
debounce_ms = 300
```

Automatic source reload can be disabled for maintenance with `reload.enabled = false`, with `domains.reload.enabled = false` for one domain, or process-wide with `--no-source-reload`.

## What is watched

The compiler returns the exact application source dependency graph: the configured entrypoint plus every direct and transitive module loaded through `mod`. The shared reload supervisor remembers those files and checks only inexpensive filesystem metadata (`mtime + size`). It does not walk every domain workdir, hash every source file, or perform source checks on every HTTP request.

The configured logical `app` path is watched in addition to canonical module paths. This matters for atomic release layouts such as:

```text
/srv/velran/domains/example.com/current -> releases/2026-09-05.2
```

Switching the `current` symlink therefore triggers a new compile even when the old dependency graph points into the previous canonical release directory.

There is one shared supervisor for all domains rather than one polling thread per domain. Each domain is checked according to its effective `poll_interval_ms`.

## Change, debounce, compile, commit

When metadata changes, Velran waits for the source set to remain stable for `debounce_ms`. This avoids recompiling once per file while a release is being copied.

The candidate application is then compiled and validated using the same hosting checks used during startup/reload, including route/resource/cache/auth constraints. Compilation runs away from request handling. On success, Velran atomically replaces that domain runtime. Requests already holding the previous runtime finish on the old generation; later requests use the new generation.

A successful source-code commit also advances the domain's public-cache generations for cached routes. Old generated HTML/JSON is therefore not kept visible merely because its previous TTL has not expired.

## Modules added, removed, or uploaded late

Adding a module is detected through its already-known parent source: adding a new `mod` declaration changes the parent file and triggers compilation. After a successful compile, the newly discovered module becomes part of the watched dependency graph automatically.

Removing or renaming a watched module is also a change. The candidate compile fails, the error is logged, and the old application generation remains active.

A multi-file upload may temporarily contain a parent module that references a new child file which has not arrived yet. In that case the first candidate compile can fail legitimately. Velran retries a stable failed candidate with exponential backoff, starting at 2 seconds and capped at 60 seconds. This lets a late module activate without continuously recompiling permanently broken code.

## Failure behavior and logging

A failed automatic reload does **not** take the domain offline. The previous valid generation remains active. Structured events include:

- `source_change_detected`
- `source_reload_committed`
- `source_reload_rejected`
- `source_reload_cache_invalidation_failed`
- `source_reload_stale`

`source_reload_rejected` includes the canonical domain, active generation, and compiler/validation error so syntax and module errors remain diagnosable from the server log. Native cdylib failures also retain the bounded real `rustc` stderr instead of replacing it with an Velran imitation. With non-production `debug_compile_errors = true` (or `--debug-compile-errors`), the detailed frontend/rustc diagnostic is additionally written as `source_reload_diagnostics` and shown as an HTML-escaped, domain-scoped developer error page for GET/HEAD requests. Health endpoints remain available and the last valid generation is not discarded. Production policy rejects detailed compiler diagnostics.

## Deployment workflow

For normal application-only deployment, the recommended sequence is:

1. build/upload the new immutable release directory;
2. optionally run `velran-server --config ... --check-config` against the intended configuration before switching traffic;
3. atomically switch the domain's `current` symlink, or update the watched application files;
4. let the source-reload supervisor compile and commit the new generation;
5. confirm `source_reload_committed`, health/readiness, and application smoke checks.

Changes to process-level settings such as listeners, database/Redis/auth connections, cgroup limits, or logging sinks still require the normal configuration/restart lifecycle. `SIGHUP` in behind-proxy mode is for transactional domain/application configuration reload; automatic source reload is the lighter application-code path.


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
