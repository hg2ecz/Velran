<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# `f32` numeric values

Velran provides Rust-like `f32` for bounded, binary floating-point computation. It is distinct from `Decimal`: use `Decimal` for exact decimal/business values and `f32` for numerical algorithms where IEEE-754 single precision is appropriate.

`f32` literals are explicit and require the `f32` suffix:

```vrn
let x = 1.25f32;
let y = -0.5f32;
let z = x * y + 2.0f32;
```

Unsuffixed fractional literals are rejected. Mixed arithmetic is also rejected: `1 + 0.5f32` is not implicitly converted. This keeps numeric intent visible and avoids accidental loss of precision.

Runtime `f32` values must be finite. `NaN` and positive/negative infinity are rejected at input boundaries, and arithmetic that would produce a non-finite result fails closed. Division by zero is a runtime error.

`f32` remains a distinct verified numeric value rather than being converted through `Decimal`. Generated safe Rust performs the supported arithmetic directly as single-precision operations, with non-finite results rejected fail-closed.

Typed HTTP/form/query input can use `f32`; textual input is parsed as finite decimal notation. Database model fields can also use `f32`; the current DB portability layer stores it canonically as text so all supported backends preserve the same parsing contract.

Bounded arrays, indexing, budgeted loops, math methods and monotonic timing are now available in the verified native subset; see the following numeric chapters and the FFT4096 example.
