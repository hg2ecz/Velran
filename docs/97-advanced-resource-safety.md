<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Advanced resource safety

Resource safety is part of Velran's security model rather than an operator-only performance concern.

## Database cardinality

Queries returning `Vec<Model>` must contain a compiler-recognized static literal row limit. Parameterized or oversized row limits are rejected. The runtime also enforces a defense-in-depth row cap.

## Whole-request deadline

Every application request executes under a platform-owned hard deadline in addition to instruction/allocation budgets. Waiting on database or outbound I/O therefore cannot escape the request lifetime budget.

## External I/O budget

The runtime tracks cumulative external I/O for relevant request input and outbound activity. Exceeding it fails as a resource-limit condition without exposing internal details.

## Compression and upstream responses

Velran does not transparently decompress untrusted request/upstream content. Non-identity `Content-Encoding` is rejected on these boundaries until a bounded decompression API exists. Named outbound calls also have a hard response-body ceiling and reject unsafe transfer-encoding behavior.

## File processing

Uploads remain staged/private until validation/publish, with file-byte and image pixel/dimension limits.

Generic unbounded collection growth is deliberately not added merely for language convenience. Any future expansion of generic `Vec<T>`/map surfaces must preserve cardinality and resource invariants.
