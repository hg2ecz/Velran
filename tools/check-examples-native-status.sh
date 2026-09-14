#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
pending="$root/tests/manifests/native-pending.txt"
tmp=$(mktemp)
trap 'rm -f "$tmp" "$tmp.expected"' EXIT
find "$root/examples" -type f -name '*.vrn' | sort | while IFS= read -r file; do
    rel=${file#"$root/"}
    if grep -Eq '#\[query\]|(^|[[:space:]])transaction[[:space:]]|@for[[:space:]]|@layout\(|@component\(|@markdown' "$file"; then
        printf '%s\n' "$rel"
    fi
done > "$tmp"
sort "$pending" > "$tmp.expected"
if ! cmp -s "$tmp" "$tmp.expected"; then
    echo "example native-status manifest is stale" >&2
    diff -u "$tmp.expected" "$tmp" >&2 || true
    exit 1
fi
if grep -R -nE '(^|[^[:alnum:]_])set[[:space:]]+[A-Za-z_]|\b(Int|Bool|F32Array|StringList|StringDict)\b' "$root/examples" "$root/start_examples" --include='*.vrn'; then
    echo "legacy Velran syntax found in examples" >&2
    exit 1
fi
printf 'examples native-status manifest: PASS (%s pending source files)\n' "$(wc -l < "$pending" | tr -d ' ')"
