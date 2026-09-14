#!/bin/sh
set -eu

grep -Fq 'v == "vec"' crates/compiler/src/expression_primary.rs
grep -Fq '"len" => CollectionLen' crates/compiler/src/rust_method_registry.rs
grep -Fq 'normalize_compound_assignment' crates/compiler/src/rust_surface_syntax.rs
grep -Fq 'crate::page_response_tail::parse' crates/compiler/src/page_statements.rs
grep -Fq 'crate::action_response_tail::parse' crates/compiler/src/action_statements.rs
grep -Fq 'let mut real = vec![0.0f32; 4096];' examples/fft4096/main.vrn
grep -Fq 'i += 1;' examples/fft4096/main.vrn
grep -Fq 'Ok(html {' examples/fft4096/main.vrn
if grep -RIn --include='*.vrn' 'arrayF32(' examples start_examples >/dev/null; then
  echo 'legacy arrayF32 constructor remains in user corpus' >&2
  exit 1
fi
if grep -RIn --include='*.vrn' -E '(^|[^.[:alnum:]_])len\(' examples start_examples >/dev/null; then
  echo 'legacy len(collection) syntax remains in user corpus' >&2
  exit 1
fi
printf '%s\n' 'Rust syntax convergence iteration 3: PASS'
