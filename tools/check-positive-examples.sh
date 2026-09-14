#!/bin/sh
set -eu

manifest='tests/manifests/example-entrypoints.txt'

if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
    if [ "${VELRAN_REQUIRE_RUST_TOOLCHAIN:-0}" = "1" ]; then
        echo "FAIL: cargo/rustc toolchain is required for positive example compilation" >&2
        exit 1
    fi
    echo "SKIP: cargo/rustc toolchain is unavailable; set VELRAN_REQUIRE_RUST_TOOLCHAIN=1 in release CI"
    exit 0
fi
[ -f "$manifest" ] || {
    echo "missing positive example manifest: $manifest" >&2
    exit 1
}

# Keep model-return examples explicit about decoded column names.
# Qualified SQL identifiers such as a.id are not model field names unless aliased.
if grep -RInE '^[[:space:]]*SELECT[[:space:]].*[A-Za-z_][A-Za-z0-9_]*\.[A-Za-z_][A-Za-z0-9_]*([[:space:]]*,|[[:space:]]*$)' examples/canonical-url 2>/dev/null | grep -v ' AS '; then
    echo 'positive example contains a qualified model projection without explicit AS alias' >&2
    exit 1
fi

# Every educational example directory containing Velran source must have one declared
# entrypoint. Compiler rejection fixtures live under tests/fixtures, outside examples.
for dir in examples/*; do
    [ -d "$dir" ] || continue
    find "$dir" -type f -name '*.vrn' -print -quit | grep -q . || continue
    if ! grep -Eq "^${dir}/((app|main)\.vrn|app/main\.vrn)$" "$manifest"; then
        echo "positive example directory is missing from $manifest: $dir" >&2
        exit 1
    fi
done

while IFS= read -r source; do
    case "$source" in
        ''|'#'*) continue ;;
    esac
    [ -f "$source" ] || {
        echo "positive example entrypoint does not exist: $source" >&2
        exit 1
    }
    printf 'checking positive example %s\n' "$source"
    cargo run --locked -q -p velran-cli -- check "$source"
done < "$manifest"
