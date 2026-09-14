#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
grep -q 'pub max_json_depth: usize' crates/language-core/src/config.rs
grep -q 'decode_json_object_bounded' crates/server/src/request_json.rs
grep -q 'JSON nesting depth exceeded' crates/server/src/request_json.rs
grep -q 'invalid or duplicate JSON field' crates/server/src/request_json.rs
grep -q 'max_json_array_items' crates/server/src/request_json.rs
if grep -R 'serde_json' -n crates/compiler/src/codegen crates/native-build/src 2>/dev/null | grep -q .; then
  echo 'generated/native build path must not depend on serde_json' >&2
  exit 1
fi
test -f examples/json-framework-boundary/app.vrn
echo 'bounded JSON boundary verification passed'
