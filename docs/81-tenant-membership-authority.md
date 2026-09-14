<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Tenant membership authority

Velran's platform authentication session now carries a trusted, bounded set of tenant membership claims.
This is the authority foundation for a later compiler-enforced `scoped by tenantId` model system.

## Security properties

- Tenant IDs are canonical ASCII identifiers with a 128-byte maximum.
- A session carries at most 64 unique memberships.
- Anonymous sessions carry no memberships.
- Local-auth memberships are stored in the local auth database, not application input.
- Changing local-auth memberships increments `auth_generation`, invalidating existing sessions.
- LDAP deployments may provide `auth.memberships_file` / `--auth-memberships-file` with `username=tenant,tenant` mappings.
- LDAP role + membership mappings are hashed into the session generation. Mapping changes invalidate persisted sessions on the next request, including Redis-backed sessions after restart.
- Memberships are exposed to the runtime only as the internal `__authMemberships` value. Application code does not receive an ambient public tenant variable yet.

## Local auth administration

```text
velran-cli auth user-add ... --tenant acme --tenant beta
velran-cli auth memberships-set --db-url-file auth-url.txt --username alice --tenant acme
```

Membership changes are authentication-authority changes and therefore revoke previously issued authenticated sessions through generation mismatch.

## LDAP/operator mapping

Configuration file:

```toml
[auth]
memberships_file = "memberships.txt"
```

`memberships.txt`:

```text
alice=acme,beta
bob=acme
```

The mapping is fail-closed: invalid tenant IDs, duplicate users, or duplicate memberships are rejected during startup.

## Why there is no `currentTenant` yet

Membership and active tenant are different concepts. Velran intentionally does not guess an active tenant from the first membership. Compiler-enforced tenant isolation builds on this authority and is documented in [Compiler-enforced tenant isolation](82-compiler-enforced-tenant-isolation.md). Membership and active-tenant selection remain distinct concepts; the platform does not guess an active tenant from the first membership.
