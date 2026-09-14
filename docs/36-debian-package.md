<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Debian package build and `dpkg -i` installation

Velran supports two installation conventions. A manual/source install uses the local-administrator prefix documented throughout the project:

```text
/usr/local/bin/velran-server
/usr/local/bin/velran-cli
/usr/local/etc/velran/server.toml
```

A Debian package is different: files owned by `dpkg` follow Debian filesystem conventions. The `.deb` therefore installs the binaries under `/usr/bin` and the conffile under `/etc/velran` rather than writing package-managed files into `/usr/local`.

## Build the package

On a Debian/Ubuntu build host with Rust and `dpkg-deb` installed:

```bash
make deb
```

Equivalent direct invocation:

```bash
tools/package-deb.sh
```

The script performs a locked release build of both public binaries and creates, by default:

```text
dist/velran_1.0.0-1_<arch>.deb
```

Useful overrides:

```bash
tools/package-deb.sh --version 1.0.0-2
tools/package-deb.sh --output-dir /tmp/packages
```

CI may package already-built binaries without invoking Cargo:

```bash
tools/package-deb.sh --skip-build --bin-dir target/release
```

## Package contents

The package installs:

```text
/usr/bin/velran-server
/usr/bin/velran-cli
/etc/velran/server.toml
/etc/velran/rate-limits.toml
/etc/velran/resource-profiles.toml
/usr/lib/systemd/system/velran.service
/usr/lib/tmpfiles.d/velran.conf
/etc/logrotate.d/velran
/usr/share/doc/velran/
```

`/etc/velran/server.toml`, the rate-limit/resource-profile policy files, and `/etc/logrotate.d/velran` are conffiles, so local operator edits are preserved by `dpkg` across upgrades in the normal Debian manner.

The package also creates the system user/group `velran` and the runtime directories `/srv/velran/data`, `/var/log/velran`, and `/run/secrets/velran` with restrictive ownership. It intentionally does **not** start the service automatically: a generic runtime package cannot know the deployed application, credentials, TLS keys, database URL, or public host.

## Install

```bash
sudo dpkg -i dist/velran_1.0.0-1_amd64.deb
```

If the local system reports unrelated dependency issues, resolve them through the package manager, for example:

```bash
sudo apt-get -f install
```

Then deploy the application and secrets, edit `/etc/velran/server.toml`, and validate it:

```bash
sudo velran-server --config /etc/velran/server.toml --check-config
```

Only after validation should the service be enabled:

```bash
sudo systemctl enable --now velran.service
```

## Direct TLS versus reverse proxy

The packaged configuration template follows the repository's direct-TLS sample on ports 80/443. The packaged unit therefore carries only `CAP_NET_BIND_SERVICE`, allowing the unprivileged `velran` user to bind those ports.

For Apache or Nginx deployments where Velran listens on an unprivileged loopback port such as `127.0.0.1:8080`, remove that capability in a site-specific systemd override. The reverse proxy remains the TLS authority and Velran should be configured with explicit `public_host` and trusted proxy CIDRs as described in the deployment chapters.
