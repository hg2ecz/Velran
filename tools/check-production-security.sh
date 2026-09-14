#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

fail() { echo "production security check: $*" >&2; exit 1; }

[ -f crates/native-runtime/src/application_runtime/mod.rs ] || fail 'application-runtime orchestrator missing'
! find crates -type f \( -name '*gccjit*' -o -name '*register_vm*' \) | grep -q . || fail 'legacy native/VM backend returned'

grep -q 'pub const RUNTIME_ABI_VERSION: u32 = 10;' crates/runtime-abi/src/lib.rs || fail 'runtime ABI v10 typed host-service contract missing'
grep -q 'pub fuel_used: u64' crates/runtime-abi/src/result.rs || fail 'native fuel accounting missing'
grep -q 'pub allocated_bytes: u64' crates/runtime-abi/src/result.rs || fail 'native allocation accounting missing'
grep -q 'struct Fuel' crates/compiler/src/codegen/emit.rs || fail 'generated Rust fuel tracker missing'
grep -q 'velran_fuel.charge' crates/compiler/src/codegen/lower.rs || fail 'generated Rust does not charge verified control-flow fuel'
grep -q 'STATUS_BUDGET_EXCEEDED' crates/compiler/src/codegen/emit.rs || fail 'generated Rust budget-exceeded result missing'
grep -q 'STATUS_MEMORY_EXCEEDED' crates/compiler/src/codegen/emit.rs || fail 'generated Rust allocation-budget result missing'
grep -q 'charge_alloc' crates/compiler/src/codegen/emit.rs || fail 'generated Rust allocation charging missing'
grep -q '#!\[forbid(unsafe_code)\]' crates/compiler/src/codegen/emit.rs || fail 'generated safe-Rust contract missing'

for forbidden in 'std::process::Command' 'std::fs::' 'std::net::' 'TcpStream' 'UdpSocket'; do
  if grep -F "$forbidden" crates/compiler/src/codegen/emit.rs crates/compiler/src/codegen/lower.rs >/dev/null; then
    fail "generated application code exposes forbidden host authority: $forbidden"
  fi
done

grep -q 'pub fn prepare(&self)' crates/native-build/src/incremental_build/mod.rs || fail 'transactional incremental prepare missing'
grep -q 'pub fn commit(&mut self' crates/native-build/src/incremental_build/mod.rs || fail 'transactional incremental commit missing'
grep -q 'self.builder.commit(prepared)' crates/native-runtime/src/application_runtime/engine.rs || fail 'activation does not commit build state transactionally'
grep -q '\.activate(ModuleGeneration::with_bindings' crates/native-runtime/src/application_runtime/engine.rs || fail 'atomic generation activation missing'
grep -q 'UnsupportedNativeShard' crates/native-build/src/incremental_build/mod.rs || fail 'unsupported native lowering must reject candidate generation'
grep -q 'WorkerRequired' crates/native-runtime/src/application_runtime/worker_policy.rs || fail 'isolated worker policy missing'
grep -q 'pub fn reap_retired' crates/native-runtime/src/runtime_generation/active.rs || fail 'retired generation maintenance missing'

unsafe_files="$(grep -R -l 'unsafe ' crates/native-runtime/src/artifact_loader crates/native-build/src/rustc_backend/abi_shim.rs 2>/dev/null || true)"
[ -n "$unsafe_files" ] || fail 'expected audited ABI/dynamic-loader unsafe boundary missing'
if grep -R -n 'unsafe ' crates/compiler/src/codegen crates/executable-ir/src crates/native-runtime/src/application_runtime >/dev/null; then
  fail 'unsafe escaped into generated/verifier/application-runtime boundary'
fi

printf '%s\n' 'production security verification passed'

"$ROOT/tools/check-language-corpus.sh"
