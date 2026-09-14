<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# FFT4096 x10000 function-call benchmark

This benchmark exercises the same 4096-point radix-2 FFT 10,000 times through a real
Velran user-defined pure function:

```rust
#[inline(never)]
fn fft4096(real: &mut [f32; 4096], imag: &mut [f32; 4096]) { ... }
```

The per-run timer covers only the `fft4096(&mut real, &mut imag)` call. Buffer reset,
input perturbation, checksum consumption and HTML rendering are outside that timer.

Security properties of this first pure-function iteration:

- only `&mut [f32; N]` parameters are accepted;
- the function body uses the verified local-compute statement subset only;
- no DB, network, filesystem, environment, process, thread, FFI or unsafe authority exists;
- duplicate mutable arguments are rejected;
- fixed array sizes are checked at compile/IR lowering time;
- fuel and allocation usage inside the helper is charged back to the caller's route budget;
- `#[inline]`, `#[inline(always)]` and `#[inline(never)]` are the only accepted Rust inline attributes.
