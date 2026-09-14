#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail(){ echo "adversarial/fuzz regression gate: FAIL: $*" >&2; exit 1; }
C='crates/compiler/src/tests/fuzz_regression_tests.rs'
H='crates/server/src/http_io/adversarial_tests.rs'
[ -f "$C" ] || fail 'compiler deterministic mutation corpus missing'
[ -f "$H" ] || fail 'HTTP adversarial corpus missing'
grep -Fq 'compile_verified_source' "$C" || fail 'compiler corpus does not cover verifier'
grep -Fq 'catch_unwind' "$C" || fail 'compiler panic assertion missing'
grep -Fq '0..1024usize' "$C" || fail 'compiler mutation corpus is too small'
grep -Fq 'Content-Length: 4\r\nContent-Length: 4' "$H" || fail 'duplicate content-length corpus missing'
grep -Fq 'Transfer-Encoding: chunked' "$H" || fail 'TE/CL smuggling corpus missing'
grep -Fq '0..2048usize' "$H" || fail 'HTTP mutation corpus is too small'
if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
    cargo test --locked -p compiler deterministic_parser_verifier_mutation_corpus_never_panics
    cargo test --locked -p velran-server request_head_smuggling_corpus_fails_closed
    cargo test --locked -p velran-server request_head_deterministic_mutation_corpus_never_panics
elif [ "${VELRAN_REQUIRE_RUST_TOOLCHAIN:-0}" = "1" ]; then
    fail 'cargo/rustc required'
else
    echo 'adversarial/fuzz regression gate: SKIP execution (static invariants PASS)'
    exit 0
fi
echo 'adversarial/fuzz regression gate: PASS'
