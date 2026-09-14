<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# IPv6-ready outbound egress

Velran V1 outbound network policy is dual-stack. Application code still does not receive a general network capability: trusted integration code selects a named target and the target config constrains host, CIDR, port and TLS.

Example:

```toml
[[target]]
name = "payments"
hosts = ["api.payments.example"]
cidrs = ["203.0.113.0/24", "2001:db8:1200::/48"]
ports = [443]
tls_required = true
```

For both A and AAAA resolution the rule is fail-closed: every candidate address must match an allowed CIDR. One disallowed IPv4 or IPv6 candidate rejects the whole resolution. After connect, the peer address must equal the chosen candidate and pass the same CIDR policy again. TLS identity verification uses the original hostname.

IPv4-mapped IPv6 addresses are normalized before policy matching. For example `::ffff:203.0.113.9` is evaluated as `203.0.113.9`, so an IPv6-mapped CIDR cannot be used as an alternate way around the target's IPv4 policy.

Special IPv6 ranges are not implicitly trusted. Loopback (`::1`), link-local (`fe80::/10`), ULA (`fc00::/7`), multicast and unspecified addresses require an explicit matching operator-configured CIDR just like any other destination.

For V1 release evidence, run a real dual-stack integration test with mixed A/AAAA answers, a denied candidate, an IPv6 peer recheck, TLS hostname verification and an IPv4-mapped address case. Repository unit tests cover the pure policy decisions, but cannot prove the host network/DNS environment.
