<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# `f32` arrays

Velran provides a bounded numeric array primitive for compute-heavy code:

```vrn
let mut samples = vec![0.0f32; 4096];
samples[0] = 1.0f32;
let first = samples[0];
let n = samples.len();
```

A bounded `f32` array is a runtime numeric container, not a database/model field type. Array allocation is charged against the request allocation budget. The current hard safety ceiling is 1,048,576 elements per array. Indexing is bounds checked; negative and out-of-range indexes fail closed.

Rust-like mutability is explicit: arrays that are modified are declared with `let mut`, and element writes use direct assignment. The compiler still verifies types, bounds and resource accounting before native lowering. Arrays are represented compactly as `f32` values rather than generic boxed values.
