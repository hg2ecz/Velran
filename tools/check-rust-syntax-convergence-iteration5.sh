#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if grep -RIn --include='*.vrn' -E '\btoF32[[:space:]]*\(' examples start_examples >/dev/null; then
  echo 'legacy toF32(...) remains in user corpus' >&2; exit 1
fi
if grep -RIn --include='*.vrn' -E '\barrayF32[[:space:]]*\(' examples start_examples >/dev/null; then
  echo 'legacy arrayF32(...) remains in user corpus' >&2; exit 1
fi
if grep -RIn --include='*.vrn' -E '(^|[^.[:alnum:]_])len[[:space:]]*\(' examples start_examples >/dev/null; then
  echo 'legacy len(collection) remains in user corpus' >&2; exit 1
fi
grep -q 'let sample = i as f32;' examples/fft4096/main.vrn
grep -q 'legacy `toF32(...)` syntax is not supported' crates/compiler/src/expression_primary.rs
grep -q 'fn parse_cast' crates/compiler/src/expression_parser.rs
echo 'Rust syntax convergence iteration 5 passed'
