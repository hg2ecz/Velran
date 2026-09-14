<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Compute `if`, explicit `f32` conversion, and FFT4096

Velran compute code supports a budgeted conditional without an implicit `else` branch:

```vrn
let mut x = 3;
if x < 10 {
    x = x + 1;
}
```

The condition must be `bool`. Statements inside the block use the same instruction and allocation budgets as `while` compute code. Locals declared inside the block are block-local at compile time; existing mutable locals use normal Rust-like assignment.

## Explicit integer to f32 conversion

``i64 as f32`` is the explicit bridge from integer control/index values to binary floating-point calculations:

```vrn
let i = 64;
let phase = i as f32 * 0.125f32;
```

There is deliberately no implicit `i64`/`f32` coercion. `as f32` is an explicit potentially lossy IEEE-754 conversion for large integers; code that requires exact integer semantics should stay in `i64`.

## FFT4096 benchmark

`examples/fft4096/main.vrn` implements a full iterative radix-2 Cooley-Tukey FFT in Velran using two bounded `f32` buffers for real and imaginary parts. It does not delegate the transform to a native FFT library.

The deterministic input signal contains a unit-amplitude tone at bin 64 and a half-amplitude tone at bin 256. For the unnormalised transform their expected magnitudes are approximately 2048 and 1024. The example reports:

- FFT elapsed time from `std::time::Instant::now()`;
- magnitude at bin 64;
- magnitude at bin 256;
- a broad correctness-window result.

The timer starts after signal generation and covers bit reversal plus all FFT stages. Compute-heavy examples require a sufficiently large instruction budget. For performance experiments separate first native-shard compilation/load from subsequent cache-hit requests.

## Native rustc backend status

The verified rustc backend can now lower the FFT4096 numeric core directly to safe Rust. The native subset includes finite `f32`, bounded `f32` arrays, bounds-checked indexing and in-place array writes, Rust-like `as f32`, `sin`, `cos`, `sqrt`, `monotonicNanos`, structured `if`/`while`, and HTML interpolation of finite `f32` results.

Native arrays have the same 1,048,576 element hard ceiling. Allocation is charged before allocation and uses fallible reservation. Generated user-derived Rust remains `#![forbid(unsafe_code)]`; array indexing is checked and no raw pointer or SIMD intrinsic is exposed to generated code. Unsupported numerical constructs are rejected before activation; there is no VM fallback path.
