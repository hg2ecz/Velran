#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail(){ echo "target-cpu: $*" >&2; exit 1; }
grep -Fq 'pub target_cpu: String' crates/native-build/src/rustc_backend/config.rs || fail 'target CPU missing from toolchain identity'
grep -Fq 'field(&mut bytes, "target_cpu", &identity.target_cpu);' crates/native-build/src/build_cache/mod.rs || fail 'target CPU missing from cache identity'
grep -Fq 'format!("target-cpu={}", config.toolchain().target_cpu)' crates/native-build/src/rustc_backend/command.rs || fail 'target-cpu rustc flag missing'
grep -Fq 'pub(super) target_cpu: Option<String>' crates/server/src/server_config_file.rs || fail 'server target_cpu config missing'
grep -Fq 'detect_target_features' crates/native-runtime/src/application_runtime/bootstrap.rs || fail 'target CPU feature fingerprint missing'
printf '%s\n' 'target CPU optimization verification passed'
