#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "native upload ABI: $*" >&2; exit 1; }
grep -q 'INPUT_UPLOAD: u32 = 10' crates/runtime-abi/src/input.rs || fail "upload wire tag missing"
grep -q 'INPUT_IMAGE: u32 = 11' crates/runtime-abi/src/input.rs || fail "image wire tag missing"
grep -q 'NativeRequestValue::Upload' crates/server/src/connection_dispatch.rs || fail "upload is not dispatched natively"
grep -q 'encode_upload_descriptor' crates/server/src/request_input.rs || fail "upload descriptor encoder missing"
grep -q 'encode_image_descriptor' crates/server/src/request_input.rs || fail "image descriptor encoder missing"
grep -q 'NativeInputType::Upload' crates/executable-ir/src/native_scalar.rs || fail "verified upload input lowering missing"
grep -q 'ScalarExpr::Field' crates/compiler/src/codegen/lower.rs || fail "native upload field lowering missing"
! grep -R 'NativeUploadUnsupported\|native upload ABI unavailable' -n crates/server >/dev/null 2>&1 || fail "obsolete upload rejection remains"
echo "native upload ABI verification passed"
