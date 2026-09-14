<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Direct typed numeric kernels

The verified numeric backend lowers pure numeric expressions to direct Rust values rather than composing `Option<T>` through every arithmetic operation. Safety semantics remain fail-closed: integer arithmetic uses checked operations, floating-point results are tested for finiteness, square-root domain errors are rejected, and dynamic collection indices are converted and bounds-checked before safe Rust indexing.

A range-proven collection access is emitted directly as safe `array[index]`. A runtime-checked access emits an explicit `index < len` guard followed by safe indexing. The typed kernel does not call the generic scalar collection helpers for reads or writes. This keeps validation on the cold failure edge and gives rustc a much simpler success path to optimize.

This optimization is general-purpose. FFT, DSP, matrix-style loops, statistics and other pure numeric Velran code use the same verified lowering path; FFT is not a compiler intrinsic.
