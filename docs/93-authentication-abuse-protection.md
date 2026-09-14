<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Authentication abuse protection

The platform-owned authentication boundary applies bounded, multi-dimensional abuse control before expensive password/MFA verification.

Default policy:

- source+principal password failures: 8 / 300s;
- one principal across sources: 16 / 300s;
- one source across principals: 64 / 300s;
- MFA/recovery failures: 5 / 300s for pair/principal, with the same source ceiling.

Counters are independent by stage (`password`, `mfa`, reserved `reset`). Successful authentication clears only the exact pair counter. It never clears source-wide or principal-wide history, so one successful login cannot erase credential-stuffing or distributed-guessing evidence.

With Redis configured, counters are shared across instances. Without Redis, the same semantics are enforced by process-local memory for single-node/development deployments.

The public failure surface remains generic (`invalid credentials`) for authentication failures. Rate-limit exhaustion is returned as `429 Too Many Requests` without exposing whether a username exists.
