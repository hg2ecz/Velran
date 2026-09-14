<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Domain types and bounded collections

Velran domain input types let an application name validation rules once and reuse them at web trust boundaries.

```vrn
type Username = String {
    length 3 32;
    pattern "^[A-Za-z0-9_]+$";
}

type PageSize = i64 {
    range 1 100;
}
```

A route can then use the domain name directly:

```vrn
route profile GET "/profiles/:username<Username>"
    query size<PageSize>
    public => profile;
```

The compiler expands the domain constraints into the route contract. The runtime validates them before entering the handler, so handler parameters arrive with validation evidence. A raw `String` still receives Velran's default `0..4096` input bound; a domain `length` constraint replaces that broad default with the application's narrower contract.

Domain types currently use `String` or `i64` as their base. `String` supports `length` and `pattern`; `i64` supports `range`. A string domain maximum cannot exceed 4096 characters. Constraint kinds cannot be duplicated. Semicolons and commas inside the constraint body are optional separators.

Domain names now keep nominal identity throughout compiler-visible data flow. `UserId` and `ArticleId` are different types even if both use `i64` as their runtime representation. The runtime erases that nominal wrapper only after compilation, so there is no wrapper allocation or serialization overhead.

## Bounded string-list construction

`splitBounded(text, delimiter, maxItems)` constructs a `Vec<String>` with an explicit fail-closed item bound:

```vrn
let tags = splitBounded(raw, ",", 32);
```

`maxItems` must be an integer literal in `1..4096`. If the input would produce more elements, evaluation fails instead of silently truncating. Requiring a literal makes the resource bound visible to the compiler, reviewer, and reader at the call site.

The older `split` remains globally capped at 4096 items for compatibility, but security-sensitive code should prefer the narrower `splitBounded` form when the domain has a known cardinality limit.
