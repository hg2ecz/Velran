<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Budgeted `while` loops and explicit local assignment

Velran supports bounded, resource-accounted compute loops for numeric work. The loop condition must be `bool`, every condition check and body statement consumes the request instruction budget, and exhaustion fails with the normal instruction-limit error instead of allowing an unbounded worker loop.

```vrn
let mut i = 0;
let mut samples = vec![0.0f32; 4096];

while i < samples.len() {
    samples[i] = 1.0f32;
    i = i + 1;
}
```

Mutable locals are declared with `let mut` and updated with normal Rust-like assignment. Static typing is preserved; assigning an `f32` expression to an `i64` local is a compile error. Array element assignment uses `array[index] = value` and remains bounds checked at runtime.

The comparison operators are `==`, `!=`, `<`, `<=`, `>`, and `>=`. Ordering is currently defined for `i64`, `f32`, and `Decimal`; equality is also available for `String` and `bool`. Mixed numeric comparison is intentionally rejected, for example `1 < 2.0f32`.

Locals introduced with `let` inside a `while` body are scoped to the compute block from the compiler's point of view. Existing outer locals may be updated only when declared mutable. Nested `while` blocks are supported and are charged to the same instruction budget.

This feature is intended for deterministic, bounded application-side computation such as numeric transforms. It does not bypass request timeouts, allocation limits, or named resource profiles.

## Native backend status

The primary rustc backend can now lower scalar `i64`/`bool` `while` and `if` compute blocks when every nested operation is part of the verified scalar subset. Native loops remain instruction-budget bounded: generated safe Rust charges a conservative local fuel tracker before each condition evaluation and executed nested statement, and returns the runtime budget-exceeded status when the request budget is exhausted.

If a loop contains a construct that is not supported by the native backend, the candidate generation is rejected and the last known-good native generation remains active. There is no VM fallback.
