<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Critical operation contracts

Critical operations let an application name a security-sensitive business action once and let the compiler enforce its required controls.

```vrn
permission BillingWrite {
    role Admin
    role BillingAdmin
}

critical Payment {
    permission BillingWrite
    mfa
    transaction
    audit
}
```

A handler opts into the contract with one short declaration:

```vrn
#[action]
fn refund(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> critical Payment {
    transaction db {
        audit Payment id action refund;
    }
    return Ok(json(true));
}
```

The route may reuse the same intent instead of repeating permission and MFA details:

```vrn
route refund POST "/billing/:id<i64>/refund"
    auth critical Payment
    => refund;
```

`auth critical Payment` compiles to the platform-owned authentication policy implied by the contract. A critical operation is always authenticated; permission and MFA requirements strengthen that baseline.

The compiler fails closed when a required transaction or audit record is missing. Audit values still obey the existing `Secret<T>` and `Sensitive<T>` leakage rules.

This keeps the normal path short: developers name the business risk, while Velran owns the security mechanics.
