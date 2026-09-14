<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Validated input flow

Velran distinguishes internal/trusted values, externally supplied values, and values that crossed a checked route boundary.

The compiler models scalar trust as:

```text
Trusted      internal/compiler/runtime-owned data
Validated    external data accepted by a typed route contract
Untrusted    external data that has not crossed a validation contract
```

Application authors do not write `Untrusted<T>` or `Validated<T>` wrappers in normal handler signatures. The route is the trust boundary.

## Safe path by default

A route such as:

```velran
route create POST "/notes"
    form title<String>
    public => create;
```

receives an implicit maximum length for `title`. The runtime checks the route contract before entering `create`, therefore `title` has validated trust inside the handler.

Explicit validation remains concise when the domain needs a narrower contract:

```velran
route create POST "/notes"
    form title<String>
    validate title length 1 160
    public => create;
```

String path parameters receive the same safe default bound, so `:slug<String>` cannot become an unbounded request-controlled string.

Typed scalars such as `Email`, `Url`, `Slug`, `i64`, `bool`, `Uuid`, `Date`, `DateTime`, and `Decimal` gain validated trust from their decoder/normalizer. A malformed value never reaches the handler.

## Mutation boundary

Transaction queries reject unvalidated scalar arguments with `SEC-DATA-003`.

```velran
transaction db {
    createAsset(tx, file.filename)?;
}
```

Upload metadata is client-controlled and remains untrusted, so this fails unless it is converted through a future explicit validation/sanitization primitive. This prevents a trusted adapter boundary such as upload metadata from silently becoming persisted application data.

Ordinary form/query/json input stays ergonomic because the route validates it before handler execution:

```velran
transaction db {
    createNote(tx, title)?; // accepted: title is Validated<String>
}
```

The authorization proof required for protected UPDATE/DELETE keys remains independent of validation proof: validation proves that an input satisfies a data contract, authorization proves that the principal may modify the specific object.

## Design rule

> Validation is a proof produced at a boundary, not a boolean convention remembered by application code.

Future collection inputs should follow the same model with bounded collection types so cardinality and per-element validation are both part of the route contract.
