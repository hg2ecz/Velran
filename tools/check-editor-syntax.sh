#!/bin/sh
set -eu

fail() {
    echo "editor syntax: $*" >&2
    exit 1
}

vim_syntax="editors/vim/syntax/velran.vim"
vim_ftdetect="editors/vim/ftdetect/velran.vim"
mc_syntax="editors/mcedit/velran.syntax"

for f in "$vim_syntax" "$vim_ftdetect" "$mc_syntax"; do
    [ -f "$f" ] || fail "missing $f"
done

grep -Fq '*.vrn' "$vim_ftdetect" || fail "Vim ftdetect does not recognize *.vrn"
grep -Fq 'Velran (*.vrn)' "$mc_syntax" || fail "mcedit syntax does not declare *.vrn"

grep -Fq '~/.local/share/mc/syntax/velran.syntax' editors/README.md || fail "mcedit user syntax path documentation is stale"
grep -Fq 'file .\*\\.vrn$ Velran' editors/README.md || fail "mcedit Syntax registration example is stale"

for kw in pub use struct impl self Self crate super match Ok Err Some None; do
    grep -Eq "(^|[[:space:]])${kw}([[:space:]]|$)" "$vim_syntax" || fail "Vim syntax missing $kw"
    grep -Eq "keyword whole ${kw}([[:space:]]|$)" "$mc_syntax" || fail "mcedit syntax missing $kw"
done

for ty in i64 f32 bool String Result Option; do
    grep -Eq "(^|[[:space:]])${ty}([[:space:]]|$)" "$vim_syntax" || fail "Vim syntax missing type $ty"
    grep -Eq "keyword whole ${ty}([[:space:]]|$)" "$mc_syntax" || fail "mcedit syntax missing type $ty"
done

# Vim treats `contains` as syntax-command grammar, so it must not be placed in a
# :syntax keyword list. It is intentionally matched separately.
grep -Fq 'syn match velranBuiltin /\<contains\>/' "$vim_syntax" || fail "Vim contains builtin workaround missing"

if command -v vim >/dev/null 2>&1; then
    tmp="${TMPDIR:-/tmp}/velran-editor-check.$$"
    trap 'rm -rf "$tmp"' EXIT HUP INT TERM
    mkdir -p "$tmp/syntax" "$tmp/ftdetect"
    cp "$vim_syntax" "$tmp/syntax/velran.vim"
    cp "$vim_ftdetect" "$tmp/ftdetect/velran.vim"
    cat > "$tmp/sample.vrn" <<'VRN'
pub struct Demo { pub value: i64 }
impl Demo {
    pub fn new(value: i64) -> Self { Self { value } }
}
VRN
    TERM="${TERM:-dumb}" vim -Nu NONE -n -es \
        "+set rtp^=$tmp" \
        '+filetype on' '+syntax on' \
        "+e $tmp/sample.vrn" '+qa!' >/dev/null 2>"$tmp/vim.err" || {
            cat "$tmp/vim.err" >&2
            fail "Vim failed to load Velran syntax"
        }
    rm -rf "$tmp"
    trap - EXIT HUP INT TERM
fi

echo "editor syntax verification passed"
