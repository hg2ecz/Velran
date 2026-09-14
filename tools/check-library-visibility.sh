#!/bin/sh
set -eu

fail() { echo "library visibility verification failed: $*" >&2; exit 1; }

test -f crates/language-core/src/visibility.rs || fail "missing shared Visibility model"
test -f crates/compiler/src/visibility.rs || fail "missing compiler visibility policy"
grep -q 'symbol_is_accessible_from' crates/language-core/src/program_visibility.rs || fail "Program access check missing"
grep -q 'declaration_visibility' crates/compiler/src/visibility.rs || fail "declaration visibility parser missing"
grep -q 'strip_member_visibility' crates/compiler/src/visibility.rs || fail "member visibility parser missing"
grep -q 'validate_public_pure_api' crates/compiler/src/visibility.rs || fail "public API leak guard missing"
grep -q 'private field' crates/compiler/src/tests/visibility_compile_tests.rs || fail "private field regression test missing"
grep -q 'private method' crates/compiler/src/tests/visibility_compile_tests.rs || fail "private method regression test missing"
grep -q 'private function' crates/compiler/src/tests/visibility_compile_tests.rs || fail "private function regression test missing"
grep -q 'pub struct Summary' examples/library-visibility/metrics.vrn || fail "canonical pub struct missing"
grep -q 'pub fn build_summary' examples/library-visibility/metrics.vrn || fail "canonical pub fn missing"
grep -q 'pub fn length_value' examples/library-visibility/metrics.vrn || fail "canonical pub method missing"
grep -q '^    hidden: i64,' examples/library-visibility/metrics.vrn || fail "canonical private field missing"
grep -q 'examples/library-visibility/main.vrn' verify.sh || fail "canonical example is not in release verification"

# Web authority must remain route/capability-owned, not Rust visibility-owned.
if grep -R -nE 'pub[[:space:]]+(page|action|query)[[:space:]]+fn' examples crates/compiler/src/tests 2>/dev/null; then
  fail "framework callables must not use pub as an authority mechanism"
fi

# Raw public impl blocks are deliberately forbidden: public methods form the API.
if grep -R -nE '^[[:space:]]*pub[[:space:]]+impl[[:space:]]' examples 2>/dev/null; then
  fail "pub impl must remain unsupported"
fi

echo 'library visibility and encapsulation verification passed'
