#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

fail=0
check_tree() {
    tree=$1
    [ -e "$tree" ] || return 0
    if grep -RInE --include='*.vrn' '(^|[^[:alnum:]_])set[[:space:]]+[A-Za-z_][A-Za-z0-9_]*(\[[^]]+\])?[[:space:]]*=|^[[:space:]]*(page|action|query|component|layout)[[:space:]]+fn[[:space:]]+|\b(Int|Bool|F32Array|StringList|StringDict)\b|Result<Void,[[:space:]]*DbError>|Result<List<|Result<[A-Za-z_][A-Za-z0-9_:]*\?,[[:space:]]*DbError>|\b[A-Z][A-Za-z0-9_]*\.[A-Z][A-Za-z0-9_]*\b' "$tree"; then
        echo "legacy Velran syntax remains under $tree" >&2
        fail=1
    fi
    # Rust-first application expressions: do not reintroduce free-function aliases
    # for operations that already have a straightforward Rust method spelling.
    legacy_expr=$(grep -RInE --include='*.vrn' '(^|[^.[:alnum:]_])(sin|cos|sqrt|abs|ln|log10|log|exp|pow|round|floor|ceil|trim|trimStart|trimEnd|lower|upper|contains|startsWith|endsWith|replace|repeat|stringLen|substring|indexOf|lastIndexOf|charAt|containsKey|dict|toF32|monotonicNanos)[[:space:]]*\(' "$tree" 2>/dev/null | grep -vE ':[0-9]+:[[:space:]]*<' || true)
    if [ -n "$legacy_expr" ]; then
        printf '%s\n' "$legacy_expr"
        echo "non-Rust convenience expression remains under $tree" >&2
        fail=1
    fi
}

check_tree examples
check_tree start_examples

# Embedded compiler/runtime programs use syntax-shaped patterns so internal
# Rust enum variants such as ValueType::Int are not false positives.
legacy_embedded=$(grep -RInE --include='*.rs' '(^[[:space:]]*(page|action|query|component|layout)[[:space:]]+fn[[:space:]]+|: Int\b|<Int>|Result<Int,|= Int \{|: Bool\b|<Bool>|Result<Bool,|^[[:space:]]*set[[:space:]]+[A-Za-z_][A-Za-z0-9_]*(\[[^]]+\])?[[:space:]]*=)' crates/*/src/tests crates/compiler/src 2>/dev/null | grep -v 'crates/compiler/src/tests/legacy_rejection_tests.rs' || true)
if [ -n "$legacy_embedded" ]; then
    printf '%s\n' "$legacy_embedded"
    echo 'legacy Velran syntax remains in embedded test programs' >&2
    fail=1
fi

# Documentation examples and terminology must describe the current language.
if grep -RInE --include='*.md' '(^[[:space:]]*set[[:space:]]+[A-Za-z_][A-Za-z0-9_]*(\[[^]]+\])?[[:space:]]*=|\b(F32Array|StringList|StringDict)\b|<[[:space:]]*(Int|Bool)[[:space:]]*>|:[[:space:]]+(Int|Bool)\b)' docs 2>/dev/null; then
    echo 'legacy Velran syntax remains in documentation examples' >&2
    fail=1
fi

grep -q 'legacy_set_assignment_is_rejected' crates/compiler/src/tests/legacy_rejection_tests.rs || { echo 'missing legacy set rejection test' >&2; exit 1; }
grep -q 'legacy_int_type_spelling_is_rejected' crates/compiler/src/tests/legacy_rejection_tests.rs || { echo 'missing legacy type rejection test' >&2; exit 1; }
grep -q 'legacy_query_return_spellings_are_rejected' crates/compiler/src/tests/legacy_rejection_tests.rs || { echo 'missing legacy query return rejection test' >&2; exit 1; }
grep -q 'rust_query_return_spellings_are_accepted' crates/compiler/src/tests/legacy_rejection_tests.rs || { echo 'missing Rust-like query return acceptance test' >&2; exit 1; }
grep -q 'value_type_parser_rejects_legacy_spellings' crates/language-core/src/values.rs || { echo 'missing legacy ValueType rejection test' >&2; exit 1; }

[ "$fail" -eq 0 ]
echo 'Velran language corpus verification: PASS'
