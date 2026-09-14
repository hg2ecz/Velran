<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Secure routing and outbound egress

Velran treats navigation and outbound networking as capabilities, not arbitrary strings.

## Typed redirects

Actions redirect to a declared GET route:

```vrn
return Ok(redirect(AccountView(user.id)));
```

The compiler resolves the route, checks that it is a GET route, checks argument count and argument types, and stores a `RouteCall` in the AST. The runtime builds the local URL from the route declaration. Raw string redirect targets are rejected (`SEC-A01-011`).

This removes open redirects from the normal language surface and also removes hand-built path/query strings from routine application code.

## Local URL runtime boundary

The runtime represents redirect locations as opaque `LocalUrl` values. `Redirect` cannot be constructed from an arbitrary `String`; a value must first satisfy the local-path invariant. This keeps trusted Rust adapters on the same fail-closed boundary as generated Velran code.

## Outbound HTTP is not ambient authority

Velran source still has no unrestricted `http.get(String)` primitive. External network access remains a trusted integration boundary. The Rust integration API now follows a capability flow:

1. `OutboundHttpsClient::capability("payments")` resolves a named egress policy.
2. `capability.endpoint("api.example.com", 443)` validates host and port against that policy.
3. `HttpsPath::new("/v1/charges")` creates a rooted, header-safe request path.
4. `client.post_json(&endpoint, &path, ...)` performs DNS/CIDR/peer/TLS/size/timeout enforcement.

The transport no longer accepts independent `target`, `host`, `port`, and path strings at the request call site. This makes policy bypass and accidental target mixing harder in trusted adapters while keeping the common integration flow concise.

## Security model

The intended invariant is:

> Local navigation is represented by route identities. External networking is represented by explicitly granted egress capabilities. Neither boundary accepts an arbitrary user-controlled URL string.

Future Velran-level integration syntax should compile to these same capabilities rather than expose a general-purpose URL fetch primitive.
