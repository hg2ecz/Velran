<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# f32 FFT4096 benchmark

This example performs a complete in-language radix-2 FFT over 4096 `f32` samples. It does not call a native FFT library.

The input contains two deterministic tones at bins 64 and 256. For an unnormalised FFT the expected magnitudes are approximately 2048 and 1024. The page reports the measured FFT section using `std::time::Instant::now()` and a broad correctness window.

Run the server with an instruction budget high enough for compute-heavy code, then open `/fft4096`. The first request may compile and load the affected native shard; compare subsequent cache-hit requests when investigating steady-state execution overhead.
