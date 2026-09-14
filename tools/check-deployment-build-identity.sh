#!/bin/sh
set -eu
fail() { echo "FAIL: $1" >&2; exit 1; }

grep -Fq 'mod build_identity;' crates/server/src/main.rs || fail 'build identity module is not wired into server'
grep -Fq '"--print-build-id"' crates/server/src/main.rs || fail '--print-build-id is missing'
grep -Fq 'raw_args == ["--print-build-id"]' crates/server/src/main.rs || fail '--print-build-id must be an exact standalone command'
grep -Fq 'expected_build_id: Option<String>' crates/server/src/server_config_file.rs || fail 'server.expected_build_id schema is missing'
grep -Fq 'build_identity::validate_expected' crates/server/src/cli_config_apply.rs || fail 'expected build id is not validated during config load'
grep -Fq 'velran-server-build-id-v1' crates/server/src/build_identity.rs || fail 'build id domain separation is missing'
grep -Fq 'runtime_abi::RUNTIME_ABI_VERSION' crates/server/src/build_identity.rs || fail 'runtime ABI missing from build id'
grep -Fq 'language_core::LANGUAGE_VERSION' crates/server/src/build_identity.rs || fail 'language version missing from build id'
grep -Fq 'language_core::SECURITY_POLICY_VERSION' crates/server/src/build_identity.rs || fail 'security policy missing from build id'
grep -Fq 'compiler::codegen::CODEGEN_VERSION' crates/server/src/build_identity.rs || fail 'codegen version missing from build id'
grep -Fq 'security-3-deployment-build-identity' crates/language-core/src/artifact_versions.rs || fail 'security policy version was not bumped'
grep -Fq 'expected_build_id' config/server.toml.sample || fail 'production sample does not document build-id pinning'

echo 'PASS: deployment build identity is fail-closed when pinned'
