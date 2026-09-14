#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
registry="$root/crates/compiler/src/rust_method_registry.rs"
tests="$root/crates/compiler/src/tests/builtin_tests.rs"

grep -Fq '"elapsed" => Elapsed' "$registry"
grep -Fq 'parse_expr("started.elapsed().as_nanos()"' "$tests"
echo "rustlike timing chain gate: ok"
