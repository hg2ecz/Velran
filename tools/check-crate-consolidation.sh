#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail() { echo "crate-consolidation: $*" >&2; exit 1; }
count=$(grep -o '"crates/[^"]*"' Cargo.toml | wc -l | tr -d ' ')
[ "$count" -eq 14 ] || fail "expected 14 workspace crates, found $count"
for old in shard-planner artifact-format build-cache rustc-backend incremental-build artifact-loader runtime-generation application-runtime codegen-rust migrations; do
  [ ! -d "crates/$old" ] || fail "obsolete crate remains: $old"
done
[ -d crates/native-build ] || fail 'native-build missing'
[ -d crates/native-runtime ] || fail 'native-runtime missing'
[ -d crates/executable-ir/src/shard_planner ] || fail 'shard planner was not absorbed by executable-ir'
grep -q '^\[workspace.dependencies\]$' Cargo.toml || fail 'workspace dependency centralization missing'
! grep -R -n 'let typed_numeric = numeric_body.is_some();' crates/compiler/src/codegen >/dev/null || fail 'obsolete typed_numeric warning source remains'
printf '%s\n' 'crate consolidation verification passed'
