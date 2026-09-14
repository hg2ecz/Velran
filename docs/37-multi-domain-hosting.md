<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# 37. Multi-domain hosting

Velran can host either one legacy application (`server.app`) or multiple host-routed applications (`[[domains]]`). The two modes are intentionally mutually exclusive.

Each domain has one primary `host` and may define any number of `aliases`. The primary host and every alias route to the same compiled application, isolated workdir, storage root, static root, resource profiles, and domain request budget. Hostnames and aliases must be unique across the whole server configuration. An unknown `Host` receives `421 Misdirected Request`; there is no default-domain fallback in multi-domain mode.

```toml
[[domains]]
host = "example.com"
aliases = ["www.example.com", "example.net", "www.example.net"]
workdir = "/srv/velran/domains/example.com/current"
app = "main.vrn"
```

Each domain has an absolute `workdir`. Application, storage, static-asset, and domain resource-profile paths are relative to that directory and may not escape it with `..`. In multi-domain mode `storage.data_root` and `static_assets.root` must be set per domain; global absolute roots are rejected.

Global `[limits]` remains the server-wide baseline. Listener/process controls such as `max_connections`, `max_header_bytes`, `max_process_memory_bytes`, and `[cgroup]` are global hard policies. A domain may override only request/runtime budgets:

- `max_body_bytes`
- `request_timeout_ms`
- `max_form_fields`
- `max_form_field_bytes`
- `max_instructions`
- `max_runtime_alloc_bytes`
- `max_concurrent_requests` (must not exceed global `max_connections`)
- `max_queued_requests`
- `queue_timeout_ms`
- `resource_profiles_file`

A domain entry may use `config_file = "/absolute/path/domain.toml"`. Values written inline in the corresponding `[[domains]]` entry override values from that include; both override global defaults. Nested domain includes are rejected. `aliases` is one list-valued setting: if it is supplied inline, the inline list replaces the included list.

## TLS certificates and SNI

Multi-domain direct TLS uses SNI certificate selection. A domain may provide its own certificate and private key:

```toml
[domains.tls]
cert_file = "/run/secrets/velran/example.com-fullchain.pem"
key_file = "/run/secrets/velran/example.com-key.pem"
```

The certificate must cover the primary `host` and every configured alias. Velran registers all of those names with the rustls SNI resolver during startup; a name not covered by the configured certificate makes startup/preflight fail instead of becoming a runtime surprise.

A global `[tls] cert_file/key_file` pair acts as the fallback certificate for domains that do not have `[domains.tls]`. This is useful for one SAN or wildcard certificate shared by several domains. A domain-specific pair overrides the global pair for that domain and all of its aliases. If direct TLS is enabled and a domain has neither a domain certificate nor a global fallback, startup fails.

In multi-domain TLS mode the HTTP `Host` is pinned to the TLS SNI hostname. This prevents a connection authenticated for one hostname from being reused to address a different configured hostname. Host/Origin validation uses the actual requested alias, so aliases work correctly for browser state-changing requests as well.

Certificate and key paths are absolute operator-controlled paths; they are not resolved under the domain workdir. Certificate renewal can therefore replace the secret files without mixing private keys into application directories. The current server loads certificates at startup, so after renewal the process must be restarted/reloaded by the service manager.

The built-in single-host HTTP redirect listener remains intentionally unavailable in multi-domain mode; perform HTTP-to-HTTPS redirects at the reverse proxy.

Use `config/server-multidomain.toml.sample` and `config/domains/domain.toml.sample` as starting points. Run `velran-server --config ... --check-config` before deployment. All domains are compiled, all resource-profile references are validated, and direct-TLS certificate/hostname mappings are checked during startup/preflight.


For the recommended Nginx/Apache deployment, Unix sockets, bounded per-domain queues, transactional SIGHUP reload, and cache single-flight behavior, see [38. Reverse-proxy application-server mode](38-reverse-proxy-application-server.md). For automatic activation of changed entrypoints and transitive modules, see [39. Automatic application source reload](39-automatic-source-reload.md).
