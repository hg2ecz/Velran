<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Bounded request collections

Velran treats external collection cardinality as a security boundary. `Vec<String>` request fields are bounded by default and may be narrowed explicitly.

```vrn
#[page]
fn search(ctx: PageContext, tags: Vec<String>) -> Result<Json, PageError> {
    return Ok(json(tags.len()));
}

route search GET "/search"
    query tags<Vec<String>>
    validate tags items 1 16
    public => search;
```

The safe default is `items 1 64` when no explicit `items` rule is present. The explicit rule replaces that default rather than stacking another limit.

## Input representation

- Query and form inputs use repeated fields: `?tags=rust&tags=web`.
- JSON inputs use a real string array: `{ "tags": ["rust", "web"] }`.
- CSV-in-a-string is not the collection contract; no manual `split()` is required for normal request collections.

JSON arrays are hard-capped during deserialization before the handler runs. Route validation may then apply a smaller domain limit.

## Security properties

- Duplicate scalar fields still fail closed.
- Repeated values are accepted only for `Vec<String>` fields.
- Request collections become `Validated<Vec<String>>` only when their route contract proves a cardinality bound.
- The platform absolute collection ceiling is 256 items; the default route ceiling is 64.
- Empty JSON string arrays are rejected in the current wire format; use at least one item when the field is present.

Velran does not expose unbounded user-data collection iteration. Budgeted `while` exists for verified compute code, but collection transforms/iteration over request-derived data must preserve or reduce the proven cardinality bound rather than erase it.
