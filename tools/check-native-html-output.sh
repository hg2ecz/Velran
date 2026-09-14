#!/usr/bin/env bash
set -euo pipefail
fail() { echo "native HTML output check: $*" >&2; exit 1; }
grep -q 'RUNTIME_ABI_VERSION: u32 = 10' crates/runtime-abi/src/lib.rs || fail 'ABI v10 missing'
grep -q 'VALUE_HTML' crates/runtime-abi/src/lib.rs || fail 'HTML result tag missing'
grep -q 'MAX_OUTPUT_BYTES' crates/runtime-abi/src/lib.rs || fail 'bounded output missing'
grep -q 'ReturnHtml' crates/executable-ir/src/native_scalar.rs || fail 'EIR HTML return missing'
grep -q 'HtmlPart::EscapedExpr' crates/executable-ir/src/native_scalar.rs || fail 'escaped HTML lowering missing'
grep -q 'OutputWriter' crates/compiler/src/codegen/emit.rs || fail 'safe bounded writer missing'
grep -q 'html_escape' crates/compiler/src/codegen/emit.rs || fail 'HTML escaping missing'
grep -q 'output: \*mut u8' crates/native-build/src/rustc_backend/abi_shim.rs || fail 'caller-owned output ABI missing'
grep -q 'STATUS_OUTPUT_TOO_SMALL' crates/server/src/app_execution.rs || fail 'bounded retry missing'
grep -q 'String::from_utf8(output)' crates/server/src/app_execution.rs || fail 'zero-copy output ownership transfer missing'
echo 'native HTML output verification passed'
