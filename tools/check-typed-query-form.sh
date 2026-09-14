#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail(){ echo "typed-query-form gate: $*" >&2; exit 1; }

grep -Fq 'velran-15-formal-effects' crates/language-core/src/artifact_versions.rs || fail 'language version not bumped'
grep -Fq 'Query<' crates/compiler/src/handler_signature.rs || fail 'Query<T> handler parsing missing'
grep -Fq 'Form<' crates/compiler/src/handler_signature.rs || fail 'Form<T> handler parsing missing'
grep -Fq 'Query<T> is only valid on #[page] handlers' crates/compiler/src/handler_signature.rs || fail 'Query<T> method boundary missing'
grep -Fq 'Form<T> is only valid on #[action] handlers' crates/compiler/src/handler_signature.rs || fail 'Form<T> method boundary missing'
grep -Fq 'unknown query struct' crates/compiler/src/route_typed_schema.rs || fail 'typed query route schema missing'
grep -Fq 'ambiguous between legacy form and Rust struct' crates/compiler/src/route_typed_schema.rs || fail 'legacy/typed form ambiguity must fail closed'
grep -Fq 'decode_urlencoded_limited' crates/server/src/http_dispatch.rs || fail 'framework-owned bounded urlencoded parser missing'
grep -Fq 'group_fields' crates/runtime/src/request_binding.rs || fail 'duplicate-field binder missing'
grep -Fq 'ABSOLUTE_COLLECTION_MAX' crates/runtime/src/request_collections.rs || fail 'bounded repeated-field collection missing'
test -f examples/typed-query-form/main.vrn || fail 'example missing'
grep -Fq 'Query<SearchParams>' examples/typed-query-form/main.vrn || fail 'typed Query<T> example missing'
grep -Fq 'Form<LoginInput>' examples/typed-query-form/main.vrn || fail 'typed Form<T> example missing'
grep -qxF 'examples/typed-query-form/main.vrn' tests/manifests/example-entrypoints.txt || fail 'positive manifest missing typed query/form example'
# Generated application shards must not own HTTP percent/form parsing.
if grep -R -E 'percent_decode|application/x-www-form-urlencoded' crates/compiler/src/codegen >/dev/null 2>&1; then
  fail 'generated shard contains request parser logic'
fi
printf '%s\n' 'typed-query-form gate: PASS'
