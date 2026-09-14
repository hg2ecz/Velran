#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
grep -q 'pub struct JsonSchema' "$root/crates/language-core/src/schema.rs"
grep -q 'parse_json_structs' "$root/crates/compiler/src/compile_pipeline.rs"
grep -q '("Json<", TypedRequestBoundary::Json)' "$root/crates/compiler/src/handler_signature.rs"
grep -q 'rewrite_request_aliases' "$root/crates/compiler/src/handler_parser.rs"
grep -q 'json EchoInput' "$root/examples/json-framework-boundary/app.vrn"
grep -q 'input: Json<EchoInput>' "$root/examples/json-framework-boundary/app.vrn"
if grep -R --include='*.rs' -n 'serde_json' "$root/crates/compiler/src/codegen" >/dev/null; then
  echo 'generated code must not depend on serde_json' >&2
  exit 1
fi
echo 'typed Json<T> request boundary verification passed'
