<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed string dictionaries

Velran provides a compute-local `BTreeMap<String,String>` collection for small deterministic maps such as metadata, parsed key/value input, lookup tables, and intermediate request processing.

```vrn
let values = BTreeMap::new();
values["name"] = "Velran";
values["mode"] = "production";

let count = values.len();
let exists = values.contains_key("name");
let name = values["name"];

values = removeKey(values, "mode");
```

The type is intentionally explicit: both keys and values are `String`. There is no implicit conversion from `i64`, `f32`, or other scalar types.

## Missing keys

Indexing is strict. `values["missing"]` is a runtime input error rather than silently producing an empty string. Use `values.contains_key(key)` before indexing when absence is expected.

## Mutation

Dictionary insertion and replacement use normal collection assignment:

```vrn
values[key] = value;
```

`removeKey(dict, key)` returns a new dictionary, so use scalar assignment when the result should replace the current value:

```vrn
values = removeKey(values, key);
```

## Limits and accounting

A dictionary is limited to 4096 entries. Keys must be non-empty and at most 1024 UTF-8 bytes. Dictionary keys, values, and collection overhead count against the runtime allocation budget.

`BTreeMap<String,String>` is compute-local. It is not a model field, database scalar, form/query binding type, or business-audit scalar.

The runtime uses deterministic key ordering internally. This avoids hash-order-dependent behaviour and keeps tests and serialized JSON stable.
