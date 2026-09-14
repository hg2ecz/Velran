#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail() { echo "native-http-dispatch: $*" >&2; exit 1; }
grep -Fq 'native_runtime: Option<&crate::native_runtime::SharedNativeRuntime>' crates/server/src/http_dispatch.rs || fail 'HTTP dispatch does not receive native runtime'
tr -d '[:space:]' < crates/server/src/app_execution.rs | grep -Fq '.dispatch_with_host(&request.handler_name,&host_api,inputs,' || fail 'request execution does not invoke ApplicationRuntime with typed host ABI'
grep -Fq 'metrics.inc_native_execution()' crates/server/src/app_execution.rs || fail 'native backend metric missing'
! grep -Fq 'execute_request_with_profiles_and_outbound' crates/server/src/app_execution.rs || fail 'HTTP dispatch still contains interpreter fallback'
grep -Fq 'native_runtime: domain.native_runtime.as_ref()' crates/server/src/connection.rs || fail 'domain native runtime is not wired into request context'
grep -Fq 'native_runtime = crate::native_runtime::initialize(app, native)' crates/server/src/backend_support.rs || fail 'native runtime is not initialized during domain preparation'
tr -d '[:space:]' < crates/native-runtime/src/application_runtime/bootstrap.rs | grep -Fq 'entry.canonicalize()' || fail 'native entry path is not canonicalized'
grep -Fq 'native-runtime = { path = "../native-runtime" }' crates/server/Cargo.toml || fail 'server does not depend on native-runtime facade'
if grep -Eq 'native-build' crates/server/Cargo.toml; then fail 'server bypasses native-runtime facade'; fi
echo 'native HTTP dispatch verification passed'
