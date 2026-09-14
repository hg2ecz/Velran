#!/bin/sh
set -eu
fail() { echo "iteration 3 runtime hot-path check: $*" >&2; exit 1; }

EMIT=crates/compiler/src/codegen/emit.rs
SCALAR=crates/compiler/src/codegen/typed_scalar_lower.rs

# HTML output must remain escaped and allocation-accounted, but safe text is emitted in chunks.
grep -q 'let mut chunk_start = 0usize;' "$EMIT" || fail 'chunked HTML escaping missing'
grep -q "direct_html_string(value: &Arc<str>" "$EMIT" || fail 'HTML string helper must borrow Arc-backed input'
grep -q 'out.push_bytes(&bytes\[chunk_start\.\.index\], state)' "$EMIT" || fail 'safe HTML chunk emission missing'
grep -q 'b.*&.*Some("&amp;")' "$EMIT" || fail 'ampersand escaping missing'
grep -q 'b.*<.*Some("&lt;")' "$EMIT" || fail 'less-than escaping missing'
grep -q 'b.*>.*Some("&gt;")' "$EMIT" || fail 'greater-than escaping missing'
grep -q 'Some("&quot;")' "$EMIT" || fail 'quote escaping missing'
grep -q 'Some("&#39;")' "$EMIT" || fail 'apostrophe escaping missing'

# Bounded split must reject oversized collections without scanning all remaining pieces.
grep -q 'take(max_items.saturating_add(1)).count()' "$EMIT" || fail 'early bounded split cap missing'
grep -q 'piece_count > max_items' "$EMIT" || fail 'bounded split rejection missing'
grep -q 'Vec::with_capacity(piece_count)' "$EMIT" || fail 'bounded split capacity reservation missing'
grep -q 'Some(Arc::new(pieces))' "$EMIT" || fail 'typed StringList ownership mismatch'

# Read-only Arc-backed scalar operations should borrow variables instead of cloning them.
grep -q 'fn borrowed_expr_source' "$SCALAR" || fail 'borrowed expression lowering missing'
grep -q 'ScalarExpr::Variable(name) => format!("&{}", local_ident(name))' "$SCALAR" || fail 'variable borrow fast path missing'
grep -q 'ScalarBuiltin::StringLen => format!("direct_string_len({})", b(0))' "$SCALAR" || fail 'borrowed string len lowering missing'
grep -q 'ScalarBuiltin::IndexOf => format!("direct_index_of({}, {}, false)", b(0), b(1))' "$SCALAR" || fail 'borrowed indexOf lowering missing'

# Optimization must not introduce ambient authority.
! grep -R -n 'std::fs::\|std::net::\|std::process::Command' crates/compiler/src/codegen >/dev/null || fail 'codegen gained ambient authority'

printf '%s\n' 'iteration 3 runtime hot-path verification passed'
