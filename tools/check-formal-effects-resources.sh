#!/bin/sh
set -eu
fail(){ echo "FAIL: $1" >&2; exit 1; }
E=crates/language-core/src/effect.rs
M=crates/executable-ir/src/model.rs
V=crates/executable-ir/src/verifier.rs
R=crates/runtime/src/execution_context.rs
B=crates/server/src/resource_profile_bootstrap.rs
A=crates/server/src/app_execution.rs

grep -Fq 'pub enum EffectClass' "$E" || fail 'EffectClass missing'
for n in Pure TimeRead Crypto DbRead DbWrite CacheRead CacheWrite OutboundHttp UploadRead StorageRead StorageWrite AuthRead SessionWrite SecurityAudit; do grep -Fq "$n" "$E" || fail "effect class $n missing"; done
grep -Fq 'pub effects: BTreeSet<EffectClass>' "$M" || fail 'verified manifest effects missing'
grep -Fq 'effects: effect_classes' "$V" || fail 'effect classes not sealed into manifest'
grep -Fq 'for effect in &handler.manifest().effects' crates/executable-ir/src/shard_planner/fingerprint.rs || fail 'effects absent from shard fingerprint'
grep -Fq 'pub max_external_io_bytes: u64' "$R" || fail 'external IO resource dimension missing'
grep -Fq 'max_external_io_bytes' "$B" || fail 'profile parser external IO limit missing'
grep -Fq '\"max_external_io_bytes\":{}' "$B" || fail 'resource profile audit log omits external IO limit'
grep -Fq 'NativeHostBridge::new(outbound, external_io_budget)' "$A" || fail 'host still derives IO budget from allocation budget'
! grep -Fq 'allocation_budget.saturating_mul(2)' "$A" || fail 'legacy coupled IO budget remains'
grep -Fq 'velran-15-formal-effects' crates/language-core/src/artifact_versions.rs || fail 'language version missing'
grep -Fq 'pub const EXECUTABLE_IR_VERSION: u16 = 25;' "$M" || fail 'EIR version missing'
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen version missing'
echo 'PASS: formal effect taxonomy, verified manifest fingerprinting, and independent external-IO resource composition are present.'
