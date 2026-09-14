#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "native-only execution: $*" >&2; exit 1; }
! grep -R "VmOnly\|NativeWithVmFallback\|VmFallbackEvent\|NativeDispatchOutcome::VmFallback\|after_native_failure\|BackendDecision" -n crates/native-runtime crates/server >/dev/null 2>&1 || fail "VM fallback policy remains in native execution crates"
! test -f crates/native-runtime/src/runtime_generation/execution_policy.rs || fail "obsolete execution policy file remains"
grep -q 'UnsupportedNativeShard' crates/native-build/src/incremental_build/mod.rs || fail "unsupported native lowering must reject candidate build"
grep -q 'map_err(IncrementalBuildError::Native)' crates/native-build/src/incremental_build/mod.rs || fail "rustc failures must reject candidate build"
grep -q 'map_err(RuntimeError::ArtifactLoad)' crates/native-runtime/src/application_runtime/engine.rs || fail "artifact load failures must reject activation"
! grep -q 'execute_request_with_profiles_and_outbound' crates/server/src/app_execution.rs || fail "normal HTTP execution still contains interpreter fallback"
! grep -q 'native_vm_fallbacks_total' crates/observability/src/metrics.rs || fail "obsolete VM fallback metric remains"
echo "native-only execution verification passed"
! grep -R -nE 'mod vm;|mod bytecode;|execute_request_with_profiles_and_outbound' crates/runtime crates/server >/dev/null 2>&1 || fail "legacy interpreter module or entrypoint returned"
! grep -q 'NativeUploadUnsupported' crates/server/src/backend_support.rs || fail "obsolete upload fail-closed gate remains"
grep -q 'NativeRequestValue::Upload' crates/server/src/connection_dispatch.rs || fail "multipart upload must enter native ABI"
grep -q 'INPUT_UPLOAD' crates/runtime-abi/src/input.rs || fail "typed upload ABI tag missing"
