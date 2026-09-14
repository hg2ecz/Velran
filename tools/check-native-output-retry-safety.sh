#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail() { echo "native output retry safety: $*" >&2; exit 1; }
grep -Fq 'enum NativeOutputPolicy' crates/server/src/app_execution.rs || fail 'output policy enum missing'
grep -Fq 'NativeOutputPolicy::None => 0' crates/server/src/app_execution.rs || fail 'scalar/no-output path must remain allocation-free'
grep -Fq 'NativeOutputPolicy::SinglePass => output_limit' crates/server/src/app_execution.rs || fail 'effectful output path must receive the full bounded output buffer'
grep -Fq '!page.needs_db && page.effects.is_empty()' crates/server/src/app_execution.rs || fail 'retry safety must require DB-free and effect-free handler metadata'
grep -Fq 'if !matches!(output_policy, NativeOutputPolicy::RetrySafe)' crates/server/src/app_execution.rs || fail 'second invocation is not guarded by retry-safe policy'
grep -Fq 'checked_add(bytes.len())' crates/compiler/src/codegen/emit.rs || fail 'generated output length arithmetic must fail closed on overflow'
if grep -Fq 'let mut no_output = []' crates/server/src/app_execution.rs; then fail 'legacy always-probe-with-empty-output path remains'; fi
echo 'native output retry safety verification passed'
