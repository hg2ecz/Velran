#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail(){ echo "direct typed scalar kernel: FAIL: $*" >&2; exit 1; }
file=crates/compiler/src/codegen/typed_scalar_lower.rs
emit=crates/compiler/src/codegen/emit.rs
[ -f "$file" ] || fail 'typed scalar lowering module missing'
grep -Fq 'pub(crate) fn eligible(body: &VerifiedScalarBody)' "$file" || fail 'eligibility verifier missing'
grep -Fq 'NativeScalarType::StringList' "$file" || fail 'StringList typed path missing'
grep -Fq 'NativeScalarType::StringDict' "$file" || fail 'StringDict typed path missing'
grep -Fq 'direct_dict_set(&mut' "$file" || fail 'direct BTreeMap mutation missing'
grep -Fq 'direct_string_replace' "$file" || fail 'direct string operations missing'
grep -Fq 'direct_html_string' "$file" || fail 'direct escaped HTML string output missing'
grep -Fq 'direct typed scalar kernel' "$emit" || fail 'emitter does not select direct typed scalar path'
grep -Fq 'scalar_enum=false; host_api=false' "$emit" || fail 'zero-Scalar/zero-host invariant marker missing'
grep -Fq 'direct_input_string' "$emit" || fail 'typed string ABI binder support missing'
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs || fail 'codegen version missing'
echo 'direct typed scalar kernel verification passed'
