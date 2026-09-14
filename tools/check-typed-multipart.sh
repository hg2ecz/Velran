#!/usr/bin/env bash
set -euo pipefail
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
grep -Fq 'Multipart<' crates/compiler/src/handler_signature.rs
grep -Fq 'TypedRequestBoundary::Multipart' crates/compiler/src/handler_signature.rs
grep -Fq 'multipart_schema' crates/language-core/src/routing.rs
grep -Fq 'prepare_native_multipart_request_for_route' crates/runtime/src/native_request.rs
grep -Fq 'file_field_name' crates/storage/src/upload.rs
grep -Fq '!csrf_verified' crates/storage/src/upload.rs
grep -Fq 'multipart UploadInput to "private"' examples/typed-multipart/main.vrn
if grep -R -F 'Scalar::Multipart' crates >/dev/null; then
  echo 'typed-multipart: generic Scalar::Multipart runtime is forbidden' >&2; exit 1
fi
if grep -R -E 'std::fs|std::net|std::process|unsafe[[:space:]]*\{' examples/typed-multipart --include='*.vrn' >/dev/null; then
  echo 'typed-multipart: ambient authority leaked into example' >&2; exit 1
fi
printf '%s\n' 'typed-multipart gate: PASS'
