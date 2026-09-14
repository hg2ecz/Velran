<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# 38. Reverse-proxy application-server mode

For production deployments behind Nginx or Apache, Velran can run as a multi-domain application server rather than as the public TLS endpoint. This deployment role is comparable to PHP-FPM, with one important difference: the proxy talks HTTP to Velran instead of FastCGI.

Use explicit backend mode:

```toml
[server]
behind_proxy = true
unix_socket = "/run/velran/velran.sock"
```

On Unix, the socket is created with mode `0660`. Velran refuses to replace an existing symlink or non-socket path. `server.unix_socket` requires `server.behind_proxy = true`; backend TLS is rejected in this mode because TLS termination belongs to the trusted reverse proxy.

Loopback TCP is also supported. When TCP backend mode is used, the listener must be loopback and `web.trusted_proxy_cidrs` must explicitly identify the proxy. Unix-socket backend mode treats the local socket transport as trusted for forwarding-header purposes.

The proxy must preserve the original Host and explicitly provide the external scheme and client address. Example Nginx settings:

```nginx
proxy_set_header Host $host;
proxy_set_header X-Forwarded-Proto $scheme;
proxy_set_header X-Real-IP $remote_addr;
proxy_set_header X-Forwarded-For $remote_addr;
proxy_pass http://unix:/run/velran/velran.sock:;
```

Velran accepts forwarding metadata only from a trusted proxy transport. Conflicting or malformed `Forwarded`, `X-Forwarded-For`, or `X-Forwarded-Proto` values are rejected instead of silently trusted. In particular, the backend is not intended to be exposed directly to untrusted clients when `behind_proxy` is enabled.

## Per-domain concurrency and queue isolation

Each domain has an independent execution pool:

```toml
[domains.limits]
max_concurrent_requests = 64
max_queued_requests = 128
queue_timeout_ms = 3000
```

A request first tries to acquire a domain execution slot immediately. If all slots are active, at most `max_queued_requests` additional requests may wait. A full queue returns `503`; a request that waits longer than `queue_timeout_ms` also returns `503`. This prevents one busy domain from creating an unbounded wait queue or consuming another domain's execution capacity.

`max_concurrent_requests` still cannot exceed global `limits.max_connections`. `max_queued_requests = 0` is valid and disables waiting: overload fails fast.

## Transactional domain/application reload

In `behind_proxy` mode, `SIGHUP` performs two operations: log files are reopened, and the domain/application configuration is re-read. Velran builds a complete candidate hosting table, recompiles applications, validates resource-profile references, route rate policies, cache TTL policy and authentication requirements, then atomically swaps the table only if the whole candidate succeeds.

A failed reload leaves the previous hosting table active. Requests that already hold an `Arc` to the previous domain runtime finish on that runtime; later requests resolve against the newly committed table.

The reload boundary is intentionally limited to application-hosting state: domain definitions/includes, aliases, app files, workdir-relative storage/static settings, domain limits and resource profiles. Listener/socket settings, database/Redis/auth connections, process/cgroup limits, logging sinks and backend TLS mode remain process-level settings and require restart.

## Cache stampede protection (single-flight)

Public page caching uses per-key request coalescing. On a miss, the first request becomes the cache filler. Concurrent requests for the exact same cache key wait on that key rather than executing the same page generation in parallel. After acquiring the key lock they check the cache again; normally they then consume the value written by the first request.

```toml
[cache]
singleflight_wait_timeout_ms = 5000
```

The wait is bounded. If filling takes too long, waiting requests return `503 cache_fill_timeout` rather than building an unbounded pile of blocked requests. The in-process lock table is opportunistically pruned when idle keys accumulate.

This single-flight mechanism coordinates concurrent tasks inside one `velran-server` process. If multiple Velran server processes/containers share the same Redis page cache, each process can still elect one local filler, so a distributed cache stampede is still possible across processes. A future distributed lease should use a tokenized Redis lock with atomic compare-and-delete; plain `SET NX` plus unconditional `DEL` is deliberately not used because it has an expiry/reacquisition race.

## Observability

Route metric keys are domain-qualified (`canonical-domain:route`), so Prometheus route counters and latency sums remain attributable to a canonical domain without creating separate series for every alias. Health/readiness requests are still Host-routed: an unknown Host never falls back to another tenant.

Use `config/server-behind-proxy.toml.sample` as the production starting point.

## Preflight and runtime error reporting

Before restart or certificate deployment, run:

```sh
velran-server --config /usr/local/etc/velran/server.toml --check-config
```

The command does not bind the application, redirect, metrics, TCP, or Unix listeners. It parses the complete configuration and domain includes, compiles every configured Velran application, validates local resource/profile roots, and builds the TLS/SNI configuration so certificate/key and configured host/alias mismatches fail before service start. Success prints `configuration OK: ...` and exits with status 0; validation failure exits non-zero and prints the error to stderr.

During normal service operation, runtime internal/database/resource-limit failures are recorded as structured server events with request ID, canonical domain, route, method, and path. Rust task/thread panics are also logged as `thread_panic`; a panicking connection task is subsequently observed by the connection `JoinSet`, so a panic is not silently treated as a normal request completion. Client-facing responses remain generic and do not expose internal error details.

## Automatic source reload

Velran can watch the source graph of every loaded application so a newly deployed `.vrn` program becomes active without a manual SIGHUP or process restart. The watcher is enabled by default and uses metadata polling rather than one watcher thread per domain:

```toml
[reload]
enabled = true
poll_interval_ms = 1000
debounce_ms = 250
```

A domain may override the global policy:

```toml
[domains.reload]
enabled = true
poll_interval_ms = 2000
debounce_ms = 300
```

At compile time the compiler returns the exact source dependency graph (entrypoint plus all direct and transitive `mod` files). A single shared supervisor periodically checks only those known files using `mtime + size`; it does not walk the workdir and it does not hash every source on every request. The configured logical entrypoint path is watched as well, so an atomic deployment that switches a `current` directory symlink to a new release is detected even though the previous compiler graph used canonical paths.

When a metadata change is observed, the supervisor waits until the observed file set has remained stable for `debounce_ms`. It then compiles a candidate runtime on a blocking worker, validates the same route-rate/cache/auth/resource-profile constraints used by startup reload, and atomically replaces only that domain runtime. Existing requests finish on the old `Arc`; new requests use the new generation.
Before the new generation is committed, Velran advances the public-cache generation for every cached route belonging to the old or candidate application. A successful source reload therefore cannot leave old generated HTML/JSON visible until its previous TTL expires; new requests either miss and regenerate under the new code or consume content generated for the new cache generation. If cache-generation invalidation fails, the code reload is rejected and the previous runtime stays active.

If compilation or validation fails, the active generation is left untouched and a structured `source_reload_rejected` event contains the canonical domain, active generation and compiler/validation error. A changed watched file resets retry state immediately. If a deployment references a brand-new module that was not part of the previous dependency graph and that file arrives after the parent source, Velran retries the failed stable candidate with exponential backoff (2 seconds up to 60 seconds) so the late file can still activate without repeatedly compiling a permanently broken source at high frequency.

Adding a new module is therefore handled naturally: the already-known parent source changes when its `mod` declaration is added, which triggers recompilation; after success the compiler returns a new dependency graph containing the added module. Removing or renaming a watched module also counts as a change and produces a compiler error while the previous generation remains live.

For controlled maintenance, automatic source reload can be disabled globally with `reload.enabled = false`, per domain with `domains.reload.enabled = false`, or from the command line with `--no-source-reload`.
