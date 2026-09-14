<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Production configuration security policy

Velran can declare deployment security requirements without embedding operator secrets or topology in application source:

```velran
production {
    https required;
    debug disabled;
    hsts required;
    database tls required;
}
```

The declaration is a security contract, not a second server-configuration system. Certificates, CIDRs, database endpoints and secret paths remain trusted operator configuration.

With the strict production policy enabled, startup fails closed when the effective deployment is incompatible, including insecure development cookies, application source reload, permissive missing-Origin behavior, remote database connections without TLS enforcement, non-HTTPS configured CORS origins or an effectively non-HTTPS public topology.

Production policy cannot be silently changed through source reload; a policy change requires process restart/operator review.
