<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# MFA elevation proofs

Velran can require recent MFA elevation at the handler boundary without exposing proof wrappers or session plumbing to application code.

## MFA-only sensitive handlers

```vrn
#[action]
fn disableMfa(
    ctx: ActionContext
) -> Result<Json, PageError> requires mfa {
    return Ok(json(true));
}

route disableMfa POST "/account/mfa/disable"
    auth mfa
    => disableMfa;
```

The compiler rejects a weaker `auth user` route. The server uses the platform-owned session and its `mfa_verified` state; applications do not implement a second MFA mechanism.

## Permission plus MFA

```vrn
permission BillingWrite {
    role Admin
    role BillingAdmin
}

#[action]
fn refund(
    ctx: ActionContext,
    id: i64
) -> Result<Json, PageError> requires BillingWrite + mfa {
    return Ok(json(true));
}

route refund POST "/billing/:id<i64>/refund"
    auth permission BillingWrite mfa
    => refund;
```

Both conditions are mandatory at runtime: the session must hold a role granting `BillingWrite` and it must be MFA-verified.

## Fail-closed rule

`requires ... + mfa` is a handler security contract, not documentation. A route that grants the permission but omits MFA fails compilation with `SEC-A07-020`.

The secure path is intentionally short: application code declares intent and the compiler/server enforce the platform session policy.
