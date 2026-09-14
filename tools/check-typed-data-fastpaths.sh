#!/bin/sh
set -eu
CG=crates/compiler/src/codegen/typed_scalar_lower.rs
EM=crates/compiler/src/codegen/emit.rs
grep -Fq 'NativeScalarType::StringList => "std::sync::Arc<Vec<std::sync::Arc<str>>>"' "$CG"
grep -Fq 'if value.is_ascii()' "$EM"
grep -Fq 'return Some(value);' "$EM"
grep -Fq 'to_ascii_uppercase()' "$EM"
grep -Fq 'to_ascii_lowercase()' "$EM"
grep -Fq 'rust-aot-50-release-stabilization' crates/compiler/src/codegen/mod.rs
! grep -Fq 'NativeScalarType::StringList => "Vec<std::sync::Arc<str>>"' "$CG"
echo "typed data fastpaths: ok"
test -f examples/typed-data-fastpaths/main.vrn
grep -Fq 'while run < runs' examples/typed-data-fastpaths/main.vrn
grep -Fq 'text.to_lowercase()' examples/typed-data-fastpaths/main.vrn
grep -Fq 'parts[1]' examples/typed-data-fastpaths/main.vrn
grep -Fq 'headers.contains_key("x-app")' examples/typed-data-fastpaths/main.vrn
grep -Fq 'Checksum:' examples/typed-data-fastpaths/main.vrn
