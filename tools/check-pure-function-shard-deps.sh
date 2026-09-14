#!/bin/sh
set -eu
fail() { echo "FAIL: $*" >&2; exit 1; }
PL=crates/executable-ir/src/shard_planner/planning.rs
CG=crates/compiler/src/codegen/mod.rs

grep -Fq 'referenced_pure_functions(program, handlers.values())' "$PL" || fail 'shards still clone every pure function'
grep -Fq 'ScalarStatement::PureCall { function, .. }' "$PL" || fail 'pure-call dependency collection missing'
# Keep this guard formatting-independent: rustfmt may split the combined match arm.
grep -Fq 'ScalarStatement::If { statements, .. }' "$PL" || fail 'nested if pure-call dependency collection missing'
grep -Fq 'ScalarStatement::While { statements, .. }' "$PL" || fail 'nested while pure-call dependency collection missing'
grep -Fq '=> collect(statements, names),' "$PL" || fail 'nested pure-call recursion missing'
! grep -Fq 'program.pure_functions().clone()' "$PL" || fail 'all-pure-functions shard cloning remains'
grep -Fq 'rust-aot-50-release-stabilization' "$CG" || fail 'codegen cache version not bumped'
grep -Fq 'format!("velran_pure_{}_{:016x}", crate_ident(value), hash)' "$CG" || fail 'lint-clean collision-resistant pure helper identifiers missing'
echo 'PASS: pure function shard dependencies are minimal and generated helper identifiers are lint-clean'
