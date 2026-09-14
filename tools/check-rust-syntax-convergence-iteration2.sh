#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
Q=crates/compiler/src/query_parser.rs
E=crates/compiler/src/expression_primary.rs
grep -q 'value == "()"' "$Q"
grep -q 'strip_prefix("Option<")' "$Q"
grep -q 'strip_prefix("Vec<")' "$Q"
! grep -q 'value == "Void"' "$Q"
! grep -q "strip_suffix('?')" "$Q"
! grep -q 'strip_prefix("List<")' "$Q"
grep -q 'v.rsplit_once("::")' "$E"
! grep -RIn --include='*.vrn' -E 'Result<Void|Result<List<|Result<[A-Za-z_][A-Za-z0-9_:]*\?,|\bList<String>|\bDict<String|\b[A-Z][A-Za-z0-9_]*\.[A-Z][A-Za-z0-9_]*\b' examples start_examples
echo 'rust syntax convergence iteration 2: PASS'
