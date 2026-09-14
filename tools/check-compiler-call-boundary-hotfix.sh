#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

ROUTES="$ROOT/crates/compiler/src/routes.rs"
PURE_CALLS="$ROOT/crates/compiler/src/pure_calls.rs"
HANDLER_SIG="$ROOT/crates/compiler/src/handler_signature.rs"

if grep -q 'UploadField' "$ROUTES"; then
  echo "FAIL: stale UploadField import/reference remains in routes.rs" >&2
  exit 1
fi

grep -Fq 'parts: &[&str]' "$PURE_CALLS" || {
  echo "FAIL: pure-call validator must accept split_top_level borrowed string slices" >&2
  exit 1
}

grep -Fq '(TypedRequestBoundary::Json, _) =>' "$HANDLER_SIG" || {
  echo "FAIL: Json<T> handler context match must be exhaustive" >&2
  exit 1
}

grep -Fq 'Json<T> requires a page or action handler context' "$HANDLER_SIG" || {
  echo "FAIL: invalid Json<T> context must fail closed" >&2
  exit 1
}

echo "compiler call/boundary hotfix: PASS"
