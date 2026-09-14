#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fail(){ echo "release-stabilization: FAIL: $*" >&2; exit 1; }
pass(){ echo "release-stabilization: PASS"; }

grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen version not bumped'
grep -Fq 'pub const ARTIFACT_MANIFEST_VERSION: u16 = 4' crates/native-build/src/artifact_format/mod.rs || fail 'manifest version not bumped'
grep -Fq 'velran-native-cache-v4-runtime-contract' crates/native-build/src/build_cache/mod.rs || fail 'cache namespace not bumped'
grep -Fq 'VELRAN_RUNTIME_CONTRACT_FINGERPRINT' crates/compiler/src/codegen/emit.rs || fail 'generated shard contract fingerprint missing'
grep -Fq 'velran_module_contract_fingerprint' crates/native-build/src/rustc_backend/abi_shim.rs || fail 'contract fingerprint export missing'
grep -Fq 'CONTRACT_SYMBOL' crates/native-runtime/src/artifact_loader/unix_abi.rs || fail 'loader contract symbol missing'
grep -Fq 'library.contract_fingerprint()' crates/native-runtime/src/artifact_loader/contract.rs || fail 'loaded module contract check missing'
grep -Fq 'LoadError::ContractMismatch' crates/native-runtime/src/artifact_loader/contract.rs || fail 'contract mismatch verifier must fail closed'
grep -Fq 'ContractMismatch { expected: u64, actual: u64 }' crates/native-runtime/src/artifact_loader/error.rs || fail 'contract mismatch error variant missing'
grep -Fq 'contract_fingerprint: u64' crates/native-build/src/artifact_format/mod.rs || fail 'manifest contract fingerprint missing'
grep -Fq 'self.contract_fingerprint != contract_fingerprint(shard)' crates/native-build/src/artifact_format/mod.rs || fail 'manifest contract verification missing'
grep -Fq 'release-o3-thinlto' crates/native-build/src/rustc_backend/config.rs || fail 'release cache identity missing thinlto'
grep -Fq '"lto=thin".into()' crates/native-build/src/rustc_backend/command.rs || fail 'release ThinLTO missing'
grep -Fq '"codegen-units=1".into()' crates/native-build/src/rustc_backend/command.rs || fail 'release single codegen unit missing'
grep -Fq '"panic=abort".into()' crates/native-build/src/rustc_backend/command.rs || fail 'panic abort missing'
grep -Fq '"overflow-checks=yes".into()' crates/native-build/src/rustc_backend/command.rs || fail 'overflow checks must remain enabled'
grep -Fq 'strip = "symbols"' Cargo.toml || fail 'workspace release symbol stripping missing'
! grep -Rqs --include='*.rs' 'Scalar::Option\|Scalar::Result\|Box<Scalar>' crates/compiler/src/codegen || fail 'dynamic sum Scalar regression'
pass
