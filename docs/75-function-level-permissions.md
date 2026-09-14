<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Function-level permissions

Velran permissions are named application capabilities backed by the roles already carried by the platform-owned authenticated session. They let application code express business intent once instead of copying role lists across routes and handlers.

## Declare a permission once

```vrn
permission UserAdmin {
    role Admin
    role SecurityAdmin
}
```

The roles inside a permission are alternatives: a session holding any listed role grants the permission.

## Require the permission at the handler boundary

```vrn
#[action]
fn removeUser(
    ctx: ActionContext,
    id: i64
) -> Result<Json, PageError> requires UserAdmin {
    return Ok(json(true));
}
```

`requires UserAdmin` is a function-level security contract. A route cannot expose this handler with merely `auth user`, `auth mfa`, an unrelated role, or `public` access.

The normal route is concise:

```vrn
route removeUser POST "/users/:id<i64>/remove"
    auth permission UserAdmin
    => removeUser;
```

The compiler resolves the permission and the server authorizes the request against the current session roles. Application code does not need to inspect role strings.

## Migration from raw roles

A route using a role that explicitly grants the required permission is accepted:

```vrn
route removeUser POST "/users/:id<i64>/remove"
    auth role Admin
    => removeUser;
```

This supports gradual migration. New code should prefer `auth permission UserAdmin` because it keeps business authorization intent stable even when role assignments change.

## Fail-closed rules

Velran rejects:

- unknown permission declarations at a handler or route boundary;
- a handler permission exposed by `public` or `auth user`;
- an unrelated role used to expose a permission-protected handler;
- empty permission declarations;
- duplicate roles inside one permission.

Important diagnostics include:

- `SEC-A01-020`: a handler requires an unknown permission;
- `SEC-A01-021`: a route references an unknown permission;
- `SEC-A01-022`: route authentication does not satisfy the handler permission.

## Security and developer experience

The permission is a compile-time domain concept while the existing session role list remains the runtime backing. This avoids a second authentication/identity system. The developer writes `UserAdmin`; the compiler and server handle role mapping and enforcement.

This is intentionally stronger than a naming convention: the handler cannot accidentally be wired to a weaker route without compilation failing.
