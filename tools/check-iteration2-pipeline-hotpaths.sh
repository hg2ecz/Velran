#!/usr/bin/env bash
set -euo pipefail

loader=crates/compiler/src/source_loader.rs
pipeline=crates/compiler/src/compile_pipeline.rs
emit=crates/compiler/src/codegen/emit.rs
typed=crates/compiler/src/codegen/typed_scalar_lower.rs
shim=crates/native-build/src/rustc_backend/abi_shim.rs
build=crates/native-build/src/rustc_backend/build.rs

grep -Fq 'namespace: String' "$loader"
grep -Fq 'pub(crate) fn namespace(&self) -> &str' "$loader"
if grep -Fq 'self.module_path.join("::")' "$loader"; then
  echo 'namespace() still allocates/join on every compiler pass' >&2
  exit 1
fi
grep -Fq 'domain_types::parse_domain_types(&u.source, u.namespace(), &mut p)' "$pipeline"
grep -Fq 'let source_name = u.path.to_string_lossy();' "$pipeline"

grep -Fq 'Scalar::String(v) => if direct_html_string(&v, out, state)' "$emit"
grep -Fq 'if from == to { return Some(Scalar::String(text)); }' "$emit"
grep -Fq 'if count == 0 { return Some(Scalar::String(text)); }' "$emit"
grep -Fq 'if from == to { return Some(text); }' "$emit"
grep -Fq 'if count==0 { return Some(text); }' "$emit"

grep -Fq 'fn direct_dict_index(items: &Arc<BTreeMap<Arc<str>,Arc<str>>>, key: &Arc<str>' "$emit"
grep -Fq 'borrowed_expr_source(body, index)' "$typed"

grep -Fq 'use std::sync::OnceLock;' "$shim"
grep -Fq "pub(crate) fn source() -> &'static str" "$shim"
grep -Fq 'SOURCE.get_or_init' "$shim"
grep -Fq 'source.push_str(abi_shim::source());' "$build"

echo 'iteration 2 compiler/native pipeline hot-path verification passed'
