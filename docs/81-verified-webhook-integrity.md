<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Verified webhook integrity boundary

Velran webhook endpoints are a distinct request authority. They are not `public` routes and they do not rely on browser sessions or CSRF tokens.

A webhook policy is declared once:

```vrn
webhook BillingEvents {
    verified by BillingWebhookKey
    signatureHeader "x-billing-signature"
    timestampHeader "x-billing-timestamp"
    replayWindow 300
}
```

The route binds that policy directly:

```vrn
route billingEvent POST "/webhooks/billing"
    json eventId<String> eventType<String>
    webhook BillingEvents
    => billingEvent;
```

The server verifies the raw request bytes before JSON decoding or handler execution. The canonical signature input is:

```text
<unix-timestamp>.<raw-request-body>
```

The signature is HMAC-SHA256 encoded as 64 hexadecimal characters. Secret material is loaded from `web.webhook_secrets_dir` using the name from `verified by ...`; secret files are bounded, non-symlink regular files and must not be accessible to group/other users on Unix.

Replay protection is platform-owned. The timestamp must be within the declared replay window (30-3600 seconds), and a verified signature/timestamp tuple is atomically claimed in Redis with `SET NX`. A duplicate claim is rejected before the handler runs. Redis and the configured secret directory are mandatory for applications containing webhook routes; startup and source reload fail closed if those dependencies are unavailable.

Webhook routes intentionally bypass browser Origin/CSRF checks only after the compiler has classified the route as a verified webhook boundary. Normal POST routes retain the existing browser state-change and CSRF enforcement.

Current surface limitation: webhook bodies use Velran's typed scalar JSON object boundary. Nested provider-specific event schemas should be added as a later typed webhook-schema feature rather than exposing a raw JSON escape hatch.
