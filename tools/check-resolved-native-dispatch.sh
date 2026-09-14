#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

loader=crates/native-runtime/src/artifact_loader/unix.rs
abi=crates/native-runtime/src/artifact_loader/unix_abi.rs
symbols=crates/native-runtime/src/artifact_loader/unix_symbols.rs
generation=crates/native-runtime/src/runtime_generation/generation.rs
binding=crates/native-runtime/src/runtime_generation/binding.rs
engine=crates/native-runtime/src/application_runtime/engine.rs

count=$(grep -c 'unsafe { dlsym' "$symbols")
if [ "$count" -ne 1 ]; then
  echo "expected one centralized dlsym site, found $count" >&2
  exit 1
fi
resolve_calls=$(grep -c 'unix_symbols::resolve(handle' "$abi")
if [ "$resolve_calls" -ne 3 ]; then
  echo "expected ABI, contract, and invoke symbols to resolve at activation, found $resolve_calls" >&2
  exit 1
fi
if awk '/pub\(super\) fn invoke\(/,/^    }/' "$loader" | grep -q 'dlsym\|resolve(handle'; then
  echo "request-time invoke still performs symbol resolution" >&2
  exit 1
fi

grep -q 'invoke: InvokeFn' "$loader"
grep -q 'unix_abi::resolve(handle)' "$loader"
grep -q 'artifact: Arc<LoadedArtifact>' "$binding"
grep -q 'HandlerBinding::new' "$engine"

invoke_body=$(awk '
  /Hot path: one verified handler-name lookup/ { capture=1 }
  capture { print }
  capture && /pub fn invoke_with_host\(/ { exit }
' "$generation")
if ! printf '%s\n' "$invoke_body" | grep -q 'get(handler_name)'; then
  echo 'generation invoke no longer resolves the handler binding by handler name' >&2
  exit 1
fi
if printf '%s\n' "$invoke_body" | grep -q 'self.shards.get'; then
  echo "generation invoke still performs a second shard-map lookup" >&2
  exit 1
fi

echo "resolved native dispatch checks PASS"
