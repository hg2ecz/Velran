<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Security architecture and current status

The canonical repository-level summary is [`../SECURITY-STATUS.md`](../SECURITY-STATUS.md).

This chapter is the documentation-index anchor for the current security model. The core principles are:

1. unsafe states should be unrepresentable when possible;
2. static proofs are preferred over runtime checks;
3. runtime security policy is platform-owned and fail closed;
4. authority is explicit and typed;
5. secrets/sensitive data preserve classification through calls and output boundaries;
6. critical operations model authorization, transaction, audit, idempotency and exceptional states together;
7. resource exhaustion is a security condition;
8. compiler/runtime security guarantees must remain testable and release-evidenced.

The seven concentrated hardening iterations completed after the earlier security foundation are:

1. closed sum types + exhaustive match;
2. transaction exceptional-state semantics + idempotency integration;
3. multi-dimensional authentication abuse protection;
4. authenticated encryption + key lifecycle;
5. database/request resource safety;
6. outbound/I/O/compression resource safety;
7. structured security monitoring + `Redacted<T>` + alerting foundation.

Future security work should require a concrete threat model or a demonstrated gap. Pure control-count/checklist expansion is not a core roadmap goal.
