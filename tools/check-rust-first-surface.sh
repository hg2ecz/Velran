#!/bin/sh
set -eu
fail() { echo "rust-first surface check: $*" >&2; exit 1; }
# Public FFT example must use Rust syntax for math and timing.
grep -q 'std::time::Instant::now()' examples/fft4096/main.vrn || fail 'Instant::now missing from FFT example'
grep -q 'elapsed().as_nanos()' examples/fft4096/main.vrn || fail 'elapsed().as_nanos missing from FFT example'
grep -q '\.sin()' examples/fft4096/main.vrn || fail 'Rust-like sin method missing'
grep -q '\.cos()' examples/fft4096/main.vrn || fail 'Rust-like cos method missing'
grep -q '\.sqrt()' examples/fft4096/main.vrn || fail 'Rust-like sqrt method missing'
if grep -E '(^|[^.[:alnum:]_])(sin|cos|sqrt|abs|ln|log10|log|exp|pow|round|floor|ceil|monotonicNanos)\(' examples/fft4096/main.vrn >/dev/null; then
  fail 'legacy duplicate math/timing surface remains in FFT example'
fi
grep -q '"sin" => Builtin(BuiltinFunction::Sin)' crates/compiler/src/rust_method_registry.rs || fail 'sin semantic method registry entry missing'
grep -q 'std::time::Instant::now' crates/compiler/src/expression_primary.rs || fail 'Instant::now lowering missing'
grep -q 'RustMethod::Elapsed' crates/compiler/src/expression_postfix.rs || fail 'elapsed semantic lowering missing'
grep -q 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen version not bumped'
printf '%s\n' 'Rust-first surface verification passed'
