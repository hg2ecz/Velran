<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed string lists

Velran now has a compute-local `Vec<String>` value. The first constructor is `splitBounded(text, delimiter, maxItems)`.

```vrn
let parts = splitBounded("alpha,beta,gamma", ",", 16);
let count = parts.len();
let first = parts[0];
```

`split` has the type:

```text
splitBounded(String, String, i64) -> Vec<String>
```

The delimiter must not be empty. A split is capped at 4096 resulting items and the resulting strings plus collection overhead count against the runtime allocation budget.

`Vec<String>` is currently a compute-local type. It is not accepted as a model field, route/form scalar, database scalar, or audit scalar. This keeps persistence and request schemas explicit while the general collection model is being built.

Indexing is checked at runtime. Negative or out-of-range indices fail closed instead of returning an implicit null value.

The collection expression layer is shared with bounded `f32` arrays:

```vrn
let samples = vec![0.0f32; 4096];
let n = samples.len();
let x = samples[0];

let words = splitBounded("one two three", " ", 16);
let m = words.len();
let word = words[0];
```

This shared Rust-like `collection.len()` and `collection[index]` representation is the base for later typed collections and dictionaries.

See `examples/string-list/main.vrn` for a runnable example.
