<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Numeric operators, `f32` math and monotonic timing

Velran provides checked numeric operators and a bounded math builtin surface for application and compute code.

## Operators

```vrn
let sum = 10 + 3;
let difference = 10 - 3;
let product = 10 * 3;
let quotient = 10 / 3;
let remainder = 10 % 3;

let left = 3 << 2;
let right = 24 >> 2;
let masked = 12 & 10;
let toggled = 12 ^ 10;
let combined = 12 | 3;

let allowed = ready && authorized;
let fallback = cached || fresh;
let denied = !allowed;
```

`+`, `-`, `*`, `/`, and `%` are defined for matching `i64`, `f32`, and `Decimal` operands. `String + String` remains string concatenation. Shift and bitwise operators (`<<`, `>>`, `&`, `^`, `|`) require `i64`. Logical `&&`, `||`, and `!` require `bool`; `&&` and `||` short-circuit the right-hand side.

Operator precedence, from tighter to looser, is: unary `!`; `* / %`; `+ -`; shifts; comparisons; bitwise `&`, `^`, `|`; logical `&&`; logical `||`. Parentheses should be used whenever they make intent clearer.

Arithmetic remains checked. Integer overflow, division/remainder by zero, non-finite `f32` results, and invalid shift counts fail closed instead of silently wrapping or producing invalid runtime values.

## `f32` math methods

```vrn
let x = 0.5f32;
let s = x.sin();
let c = x.cos();
let root = 4.0f32.sqrt();
let magnitude = (-3.5f32).abs();
let natural = 2.0f32.ln();
let decimal = 1000.0f32.log10();
let base2 = 8.0f32.log(2.0f32);
let exponential = 1.0f32.exp();
let powered = 2.0f32.powf(8.0f32);
let nearest = 2.6f32.round();
let down = 2.6f32.floor();
let up = 2.1f32.ceil();
```

Signatures:

```text
f32.sin() -> f32
f32.cos() -> f32
f32.sqrt() -> f32
abs(i64) -> i64
f32.abs() -> f32
f32.ln() -> f32
f32.log10() -> f32
f32.log(f32) -> f32
f32.exp() -> f32
f32.powf(f32) -> f32
f32.round() -> f32
f32.floor() -> f32
f32.ceil() -> f32
i64 as f32
```

All `f32` results must remain finite. Domain errors such as `(-1.0f32).sqrt()` or `(-1.0f32).ln()` fail instead of creating NaN or infinity.

## Monotonic timing

```vrn
let started = std::time::Instant::now();
// measured work
let elapsed = started.elapsed().as_nanos();
```

`std::time::Instant::now()` returns an `i64` nanosecond counter relative to a process-local monotonic origin. It is suitable for elapsed-time measurements, not timestamps. Pages whose public-cache output depends on this clock are rejected by the compiler.

Math builtins have weighted instruction-budget costs. Transcendental operations cost more than basic arithmetic so compute-heavy code remains governed by the configured resource limits.

See `examples/numeric-operators/main.vrn` for a runnable operator and math example.

## Related language rules

Every `let`, assignment, and `return` shown above is a simple statement and therefore ends with `;`. Newlines inside an expression do not terminate it. See [Statement terminators](56-statement-terminators.md). String concatenation with `+` and the Unicode-aware String API are documented in [String builtins](46-string-builtins.md).
