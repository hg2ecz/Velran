#!/bin/sh
set -eu
fail() { echo "FAIL: $*" >&2; exit 1; }

if grep -Fq '#[cfg(test)]' crates/native-build/src/rustc_backend/build.rs; then
  grep -Fq 'use crate::rustc_backend::ToolchainIdentity;' crates/native-build/src/rustc_backend/build.rs \
    || fail 'build.rs nested tests must import ToolchainIdentity from rustc_backend'
fi
grep -Fq 'use crate::rustc_backend::{OptimizationProfile, ToolchainIdentity};' crates/native-build/src/rustc_backend/diagnostics.rs \
  || fail 'diagnostics nested tests must import rustc_backend types from parent module path'
if grep -Fq 'use super::super::*;' crates/native-build/src/incremental_build/tests/release_native_smoke.rs; then
  fail 'release native smoke must not keep unused super::super::* import'
fi

echo 'native-build test-scope hotfix: PASS'
