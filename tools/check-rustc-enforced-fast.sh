#!/bin/sh
set -eu

require_grep() {
  pattern=$1
  file=$2
  message=$3
  if ! grep -q -- "$pattern" "$file"; then
    echo "rustc-enforced-fast: $message" >&2
    exit 1
  fi
}

reject_grep() {
  pattern=$1
  file=$2
  message=$3
  if grep -q -- "$pattern" "$file"; then
    echo "rustc-enforced-fast: $message" >&2
    exit 1
  fi
}

require_grep 'required = true' config/server.toml.sample 'native.required=true missing from sample server config'
require_grep 'required = true' start_examples/velran-server.toml 'native.required=true missing from starter server config'
require_grep 'NoNativeShards' crates/native-runtime/src/application_runtime/bootstrap.rs 'required-native bootstrap rejection missing'
require_grep 'native_shards={} required={}' crates/server/src/native_runtime.rs 'native shard diagnostics missing'
require_grep 'VelranInputValue::EMPTY' crates/native-runtime/src/artifact_loader/mod.rs 'empty-input ABI fast path missing'
reject_grep 'Vec<VelranInputValue>' crates/native-runtime/src/artifact_loader/mod.rs 'artifact loader regressed to Vec<VelranInputValue> allocation'

command_file=crates/native-build/src/rustc_backend/command.rs
require_grep 'codegen-units=1' "$command_file" 'release codegen-units=1 missing from rustc command builder'

command_compact=$(tr -d '[:space:]' < "$command_file")
case "$command_compact" in
  *'"--crate-type".into(),"cdylib".into()'*) ;;
  *) echo 'rustc-enforced-fast: cdylib crate type is not enforced' >&2; exit 1 ;;
esac

production_tmp=$(mktemp)
trap 'rm -f "$production_tmp"' EXIT HUP INT TERM
awk '/^#\[cfg\(test\)\]/{exit} {print}' "$command_file" > "$production_tmp"
reject_grep '"--extern"' "$production_tmp" 'native shard rustc invocation must not inject --extern dependencies'
reject_grep '"rlib".into()' "$production_tmp" 'native shard rustc invocation must stay cdylib-only'

require_grep 'DEFAULT_NATIVE_SHARD_BUCKETS: u32 = 4' crates/executable-ir/src/shard_planner/planning.rs 'default native shard bucket count changed'
require_grep 'HandlerBinding' crates/native-runtime/src/runtime_generation/generation.rs 'native handler binding missing'
require_grep 'ReturnHtml' crates/executable-ir/src/native_scalar.rs 'native HTML return path missing'
require_grep 'VELRAN_VALUE_HTML' crates/compiler/src/codegen/emit.rs 'HTML ABI value emission missing'
require_grep 'STATUS_OUTPUT_TOO_SMALL' crates/server/src/app_execution.rs 'bounded native output retry status missing'

echo 'rustc-enforced fast native path verification passed'
