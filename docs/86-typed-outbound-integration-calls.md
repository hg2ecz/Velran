<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed outbound integration calls

Velran outbound networking is capability-oriented. Application code cannot construct an arbitrary HTTP origin. A source module declares a named integration and maps it to an operator-owned egress target:

```vrn
integration Billing {
    egress payments
}
```

A handler must explicitly opt into that integration at its boundary:

```vrn
#[action]
fn charge(ctx: ActionContext, amount: i64)
    -> Result<Json, PageError>
    uses Billing
{
    let status = Billing.postJson("/v1/charges", amount)?;
    return Ok(json(status));
}
```

The compiler infers `net.payments` from the actual call and requires the matching `uses Billing` capability. Declaring `uses` without a call remains visible as declared capability metadata, while a call without `uses` is rejected.

## Security invariants

- There is no generic `http.get(userUrl)` surface.
- Scheme, host, port, DNS policy, CIDRs and TLS are not application-source authority.
- Request paths are compiler-known rooted relative string literals.
- Absolute URLs, protocol-relative paths, dynamic paths and header-control characters are rejected.
- Page handlers may issue GET calls only. State-changing `postJson` is restricted to actions.
- `postJson` accepts only compiler-approved public scalar/collection data. Sensitive, Secret, credential-purpose, Upload and implicit whole-model flows are rejected.
- The runtime receives only the integration egress target and the validated relative path.
- The server resolves the target through the trusted egress policy and refuses startup if an outbound effect exists without configured policy.
- An Velran-callable egress target must resolve to exactly one configured host and one configured port. This prevents source code from gaining endpoint-selection authority through a broad target.
- The existing integration transport keeps TLS, DNS answer limits, CIDR checks, connected-peer verification, request/response byte limits and connect/total timeout enforcement.
- Source reload revalidates outbound target availability against the active policy and fails closed.

## Operator configuration

The server points to a trusted egress policy outside the Velran application source:

```toml
[web]
egress_policy_file = "/etc/velran/egress.toml"
```

A target intended for direct Velran calls must have one host and one port, for example:

```toml
[[target]]
name = "payments"
hosts = ["api.example-payments.com"]
cidrs = ["203.0.113.0/24"]
ports = [443]
tls_required = true
max_dns_answers = 8
max_sent_bytes = 262144
max_received_bytes = 2097152
connect_timeout_ms = 5000
total_timeout_ms = 15000
```

The CIDR shown above is documentation-only; production operators must configure the real approved network ranges for the integration.

## Response trust boundary

The first call surface intentionally returns only the HTTP status code. Response bodies are not automatically parsed or treated as trusted application data. A future typed response-schema feature should introduce external response data as untrusted and require explicit validation/refinement before security-sensitive use.

Similarly, application-provided bearer tokens or arbitrary secret headers are not part of this surface. Secret-bearing integrations should use purpose-specific trusted capabilities rather than generic string headers.
