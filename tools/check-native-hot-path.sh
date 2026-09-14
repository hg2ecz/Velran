#!/bin/sh
set -eu

grep -q 'prepare_native_request_for_route' crates/server/src/app_execution.rs
grep -q 'let mut input_storage = \[RequestValue::Int(0); MAX_INPUT_FIELDS\];' crates/server/src/app_execution.rs
if grep -q 'let inputs: Vec<RequestValue' crates/server/src/app_execution.rs; then
  echo 'native hot path must not heap-allocate ABI input vector' >&2
  exit 1
fi
if grep -q 'execution_backend.*backend=native' crates/server/src/app_execution.rs; then
  echo 'successful native hot path must use metrics, not per-request JSON logging' >&2
  exit 1
fi
grep -q 'inc_native_execution' crates/server/src/app_execution.rs
grep -q 'velran_native_executions_total' crates/observability/src/metrics.rs
if grep -q 'velran_native_vm_fallbacks_total' crates/observability/src/metrics.rs; then echo 'obsolete VM fallback metric remains' >&2; exit 1; fi
grep -q 'env.remove(&param.name)' crates/runtime/src/native_request.rs
grep -q 'bind_route_path(program, route, path)' crates/runtime/src/native_request.rs
# Iteration 16: native symbols and shard selection must be activation-time work.
./tools/check-resolved-native-dispatch.sh
printf '%s\n' 'native hot-path verification passed'
