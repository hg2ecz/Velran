<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Compiler-enforced tenant isolation

Velran tenant isolation builds on the platform-owned authenticated membership claim. A tenant scope is not a free-form String check and it is not inferred from the first membership.

## Scoped model

```vrn
type OrganizationId = String {
    length 1 64;
    pattern "^[a-z0-9_-]+$";
}

model Article scoped by organizationId {
    id: i64
    organizationId: OrganizationId
    title: String
}
```

`scoped by organizationId` makes `organizationId` part of the model's security contract. The field must exist and have String representation, preferably through a nominal domain type.

## Active tenant route proof

```vrn
route article GET "/org/:organizationId<OrganizationId>/articles/:id<i64>"
    tenant organizationId
    auth user
    => article;
```

The route must be authenticated. At runtime, before the handler runs, Velran checks that the selected tenant exists in the trusted `__authMemberships` session claim. A non-member receives `Forbidden`.

Tenant selection may only come from a path parameter or a GET query parameter. It cannot come from a POST body, so tenant authority is established before request-body processing.

## Query enforcement

A query returning or mutating a scoped model automatically becomes tenant-scoped. No extra query annotation is required:

```vrn
#[query]
fn loadArticle(
    db: Db,
    organizationId: OrganizationId,
    id: i64
) -> Result<Article, DbError> sql {
    SELECT id, organizationId, title
    FROM articles
    WHERE organizationId = :organizationId
      AND id = :id
}
```

The compiler requires:

- a query parameter with the same name as the model tenant field;
- exact nominal type equality with the tenant field;
- an SQL tenant guard (`field = :field`) for SELECT/UPDATE/DELETE;
- the corresponding column/value pair for INSERT;
- a call-site argument carrying the active-route-tenant proof.

A validated `OrganizationId` from an ordinary request field is not sufficient. It must originate from the route's `tenant organizationId` binding and be passed unchanged.

## Security properties

The model prevents common cross-tenant IDOR/BOLA failures:

- forgetting the tenant predicate in SQL is a compile error;
- passing another organization ID is a compile error;
- making a tenant-bound route public is a compile error;
- using a body field as tenant authority is a compile error;
- runtime membership is checked against the authenticated session before handler execution.

This is intentionally fail-closed. Cross-tenant joins or operations involving multiple scoped models should be decomposed into explicitly scoped operations until Velran gains a richer multi-scope capability model.
