<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Velran starter project

This directory is the production-oriented starter application. It is not a set of universal deployment defaults: operators must provide the real hostnames, secrets, limits, and OS paths for their environment.

## Development

```bash
cargo run --locked -q -p velran-cli -- check examples/starter-project/main.vrn
```

The application uses `main.vrn` plus explicitly declared namespaced source files. `mod models;` loads `models.vrn` as the `models` namespace; it does not inject `Article` or other declarations into the global scope. Cross-module references therefore use qualified names such as `models::Article` and `queries::articleBySlug(...)`. Database migration is a separate deployment operation; the application server never performs schema migration automatically.

## Recommended release layout

```text
/srv/velran/releases/2026-09-04.1/
  main.vrn
  models.vrn
  queries.vrn
  pages.vrn
  actions.vrn
  migrations/
  public/
/srv/velran/current -> /srv/velran/releases/2026-09-04.1
```

Release directories are read-only. Writable application data belongs under a separate `/srv/velran/data` tree.

## Service/config authority model

The included direct-TLS configuration binds ports 80/443 while the process runs as the unprivileged `velran` user. The paired systemd unit therefore grants only `CAP_NET_BIND_SERVICE`; running the service as root is not required. When deploying behind Apache or Nginx on `127.0.0.1:8080`, remove that capability.

In this starter, systemd is the authority for process-level cgroup ceilings (`MemoryMax`, `MemorySwapMax`, `CPUQuota`, `TasksMax`). The supplied `server.toml` therefore does not enable Velran's own cgroup writer. Do not use both authorities at the same time.

## Production sequence

1. build and run `./verify.sh`;
2. record the artifact SHA-256;
3. verify database backup and restore readiness;
4. run `deploy/preflight.sh`;
5. run `migrate apply` using a dedicated migration credential;
6. install the new immutable release;
7. atomically switch the `current` symlink;
8. perform a controlled `systemctl restart velran`;
9. check `/health/live` and `/health/ready`;
10. inspect logs, metrics, and audit evidence.

Application artifacts may be rolled back, but schema rollback must never be assumed automatically. Plan database compatibility before deployment.

The previous Hungarian README is available as `README_hu.md`.
