#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
emit="$root/crates/compiler/src/codegen/emit.rs"
typed="$root/crates/executable-ir/src/typed_numeric.rs"
lower="$root/crates/compiler/src/codegen/typed_lower.rs"
lib="$root/crates/compiler/src/codegen/mod.rs"

grep -Fq 'fn velran_handler_{handler_id}(inputs: &[InputValue' "$emit"
grep -Fq 'fn velran_kernel_{handler_id}' "$emit"
grep -Fq 'direct_typed_input_binder' "$emit"
grep -Fq 'generic_input_abi=false; host_api=false' "$emit"
grep -Fq 'TypedStatement::ReturnHtml' "$typed"
grep -Fq 'TypedHtmlPart::Escaped' "$lower"
grep -Fq 'rust-aot-50-release-stabilization' "$lib"

# Pure kernels must not take HostApi or InputValue; both stay in the ABI adapter.
if grep -F 'fn velran_kernel_{handler_id}' "$emit" | grep -Eq 'HostApi|InputValue'; then
    echo 'pure typed kernel must not expose HostApi or InputValue' >&2
    exit 1
fi
# Dispatcher must construct HostApi only for the non-numeric/effectful arm.
if grep -F 'if body.numeric_body().is_some()' "$emit" | grep -Fq 'HostApi::new'; then
    echo 'pure numeric dispatcher must not construct HostApi' >&2
    exit 1
fi

echo 'pure typed native kernel boundary verification passed'
