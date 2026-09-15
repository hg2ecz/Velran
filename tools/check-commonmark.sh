#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

fail() { printf '%s\n' "commonmark: $*" >&2; exit 1; }

[ -f common/commonmark.vrn ] || fail 'missing common/commonmark.vrn'
grep -Fq 'pub fn render(source: &str) -> SafeHtml' common/commonmark.vrn || fail 'renderer public API missing'
grep -Fq 'splitBounded(normalized, "\n", 2048)' common/commonmark.vrn || fail 'renderer line bound missing'
grep -Fq 'safeHtmlText' common/commonmark.vrn || fail 'renderer must escape text through SafeHtml boundary'
grep -Fq 'safeHtmlLink' common/commonmark.vrn || fail 'renderer safe link construction missing'
grep -Fq 'ordered_content_offset' common/commonmark.vrn || fail 'ordered-list parsing missing'
grep -Fq 'list_kind = "ol".to_string();' common/commonmark.vrn || fail 'ordered-list rendering missing'
grep -Fq 'raw HTML is never passed through' common/commonmark.vrn || fail 'raw-HTML security policy missing'
grep -Fq 'fn render_blocks(source: &str, depth: i64) -> SafeHtml' common/commonmark.vrn || fail 'block parser missing'
grep -Fq 'fn render_inline(input: &str) -> SafeHtml' common/commonmark.vrn || fail 'inline parser missing'
grep -Fq 'code_fence_len' common/commonmark.vrn || fail 'fence-length tracking missing'
grep -Fq 'setext_level' common/commonmark.vrn || fail 'Setext heading parsing missing'
grep -Fq 'safe_link_target' common/commonmark.vrn || fail 'link policy missing'
grep -Fq 'value.starts_with("//")' common/commonmark.vrn || fail 'protocol-relative URL rejection missing'
grep -Fq 'ascii_punctuation' common/commonmark.vrn || fail 'CommonMark backslash escape policy missing'
grep -Fq 'safeHtmlElement("br"' common/commonmark.vrn || fail 'hard-break rendering missing'

# Markdown syntax policy must not live in compiler/runtime production Rust.
if grep -R -i --include='*.rs' 'markdown' crates/compiler/src crates/language-core/src crates/executable-ir/src crates/runtime/src crates/native-runtime/src \
    | grep -v '/tests/' \
    | grep -v 'markdown_directive_is_not_an_engine_feature' >/dev/null 2>&1; then
    fail 'Markdown-specific production Rust remains in engine/compiler/runtime'
fi

# The obsolete directive is retained only as a negative regression test/documentation statement.
if grep -R -F '@markdown' crates examples --exclude='presentation_compile_tests.rs' >/dev/null 2>&1; then
    fail 'positive @markdown usage remains in source/examples'
fi

grep -Fq 'markdown_directive_is_not_an_engine_feature' crates/compiler/src/tests/presentation_compile_tests.rs \
    || fail 'missing negative regression test for removed @markdown directive'

# SafeHtml must not be accepted by the general scalar/input type parser.
if grep -Fq '"SafeHtml" => Some' crates/language-core/src/values.rs; then
    fail 'SafeHtml is forgeable through general ValueType input parsing'
fi
grep -Fq 'domain type name `SafeHtml` is reserved' crates/compiler/src/domain_types.rs \
    || fail 'SafeHtml reserved-name guard missing'

# Both native codegen paths must preserve trusted SafeHtml without re-escaping it.
grep -Fq 'NativeHtmlPart::Safe(expr)' crates/compiler/src/codegen/lower.rs \
    || fail 'generic codegen SafeHtml branch missing'
grep -Fq 'NativeHtmlPart::Safe(expr) =>' crates/compiler/src/codegen/typed_scalar_lower.rs \
    || fail 'typed scalar codegen SafeHtml branch missing'
grep -Fq 'velran_output.push({value}.as_ref()' crates/compiler/src/codegen/typed_scalar_lower.rs \
    || fail 'typed scalar SafeHtml path must emit trusted fragment directly'

for example in markdown markdown-sql-cache commonmark wiki domain-objects news-site; do
    [ -f "examples/$example/commonmark.vrn" ] || fail "example $example missing source-level commonmark.vrn"
    cmp -s common/commonmark.vrn "examples/$example/commonmark.vrn" \
        || fail "example $example commonmark.vrn drifted from canonical common module"
done

printf '%s\n' 'source-level commonmark verification passed'
