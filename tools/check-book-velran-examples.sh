#!/bin/sh
set -eu

python3 - <<'PY'
from pathlib import Path
import re
import sys

root = Path('.')
en_dir = root / 'docs/book/chapters'
hu_dir = root / 'docs/book/hu/chapters'
errors = []

legacy_patterns = [
    (re.compile(r'\b(?:page|action|query)\s+fn\b'), 'legacy callable syntax'),
    (re.compile(r'\barrayF32\s*\('), 'legacy arrayF32 constructor'),
    (re.compile(r'\blen\s*\('), 'legacy len(...) syntax'),
    (re.compile(r'\bList\s*<'), 'legacy List<T> type'),
    (re.compile(r'\bF32\b'), 'legacy F32 type'),
    (re.compile(r'\bVoid\b'), 'legacy Void type'),
]

block_re = re.compile(r'\\begin\{lstlisting\}(?:\[([^\]]*)\])?\n(.*?)\\end\{lstlisting\}', re.S)

def velran_blocks(path: Path):
    text = path.read_text(encoding='utf-8')
    return [body for opts, body in block_re.findall(text) if 'language=Velran' in (opts or '')]

# 1. Rust-first surface: reject known legacy forms inside Velran listings.
for path in list(en_dir.glob('*.tex')) + list(hu_dir.glob('*.tex')):
    for body in velran_blocks(path):
        for pat, label in legacy_patterns:
            if pat.search(body):
                errors.append(f'{path}: {label} found in Velran listing')

# 2. Every route declaration shown as Velran must have an explicit access policy.
for path in list(en_dir.glob('*.tex')) + list(hu_dir.glob('*.tex')):
    for body in velran_blocks(path):
        for decl in re.findall(r'(?ms)^\s*route\s+.*?;', body):
            flat = ' '.join(decl.split())
            if not re.search(r'\s(?:public|auth\s+(?:user|mfa|role|permission|critical)|webhook\s+)\b', flat):
                errors.append(f'{path}: route without explicit access policy: {flat[:180]}')

# 3. English/Hungarian chapter pairs should carry the same number of Velran listings.
def chapter_map(directory: Path):
    out = {}
    for p in directory.glob('*.tex'):
        m = re.match(r'(\d\d)-', p.name)
        if m:
            out[m.group(1)] = p
    return out

en = chapter_map(en_dir)
hu = chapter_map(hu_dir)
for key in sorted(set(en) | set(hu)):
    if key not in en or key not in hu:
        errors.append(f'chapter {key}: missing English/Hungarian counterpart')
        continue
    en_n = len(velran_blocks(en[key]))
    hu_n = len(velran_blocks(hu[key]))
    if en_n != hu_n:
        errors.append(f'chapter {key}: Velran listing count differs: EN={en_n}, HU={hu_n}')

# 4. Repository example paths named in either edition must exist.
example_re = re.compile(r'examples/[A-Za-z0-9_./-]+')
for path in list(en_dir.glob('*.tex')) + list(hu_dir.glob('*.tex')):
    text = path.read_text(encoding='utf-8')
    for match in example_re.finditer(text):
        rel = match.group(0).rstrip('.,);:]')
        if not (root / rel).exists():
            errors.append(f'{path}: referenced example path does not exist: {rel}')

# 5. The canonical-example manifest is the single mapping from book chapters to
# compiler-checked copy/paste sources. Every row must exist, be referenced in both
# language editions of that chapter, and participate in verify.sh.
manifest = root / 'docs/book/CANONICAL_EXAMPLES.tsv'
verify_text = (root / 'verify.sh').read_text(encoding='utf-8')
if not manifest.exists():
    errors.append('docs/book/CANONICAL_EXAMPLES.tsv is missing')
else:
    seen = set()
    for lineno, raw in enumerate(manifest.read_text(encoding='utf-8').splitlines(), 1):
        if not raw.strip() or raw.startswith('#'):
            continue
        parts = raw.split('\t')
        if len(parts) != 3:
            errors.append(f'{manifest}:{lineno}: expected chapter, path, purpose')
            continue
        chapter, rel, purpose = parts
        key = (chapter, rel)
        if key in seen:
            errors.append(f'{manifest}:{lineno}: duplicate canonical example {chapter} {rel}')
        seen.add(key)
        if chapter not in en or chapter not in hu:
            errors.append(f'{manifest}:{lineno}: unknown chapter {chapter}')
            continue
        target = root / rel
        if not target.is_file():
            errors.append(f'{manifest}:{lineno}: canonical example does not exist: {rel}')
        for edition, chapter_path in [('EN', en[chapter]), ('HU', hu[chapter])]:
            chapter_text = chapter_path.read_text(encoding='utf-8')
            if rel not in chapter_text:
                errors.append(f'{chapter_path}: canonical example missing from {edition} chapter: {rel}')
        if rel not in verify_text:
            errors.append(f'verify.sh does not compiler-check canonical book example: {rel}')

# 6. Learning-path guidance must stay present and structurally paired.
en_preface = (en_dir / '00-preface.tex').read_text(encoding='utf-8')
hu_preface = (hu_dir / '00-eloszo.tex').read_text(encoding='utf-8')
for phrase in ['Choose a learning path', 'Fast web-developer path', 'Complete Velran path']:
    if phrase not in en_preface:
        errors.append(f'English preface missing learning-path guidance: {phrase}')
for phrase in ['Válassz tanulási útvonalat', 'Gyors webfejlesztői út', 'Teljes Velran út']:
    if phrase not in hu_preface:
        errors.append(f'Hungarian preface missing learning-path guidance: {phrase}')

for key in sorted(set(en) | set(hu)):
    if key == '00' or key not in en or key not in hu:
        continue
    en_text = en[key].read_text(encoding='utf-8')
    hu_text = hu[key].read_text(encoding='utf-8')
    if r'\textbf{Learning path:' not in en_text:
        errors.append(f'{en[key]}: missing learning-path marker')
    if r'\textbf{Tanulási út:' not in hu_text:
        errors.append(f'{hu[key]}: missing learning-path marker')

# 6b. Editorial structure: preface learning material has a stable beginner-first order,
# and EN/HU chapter pairs keep the same section/subsection shape.
def positions(text, phrases):
    return [text.find(p) for p in phrases]

en_preface_order = [
    r'\section*{Core terms}',
    r'\section*{How to read this book}',
    r'\section*{Reuse your Rust knowledge}',
    r'\section*{Minimum Rust for Velran}',
    r'\section*{Velran in 60 minutes}',
    r'\section*{Ten security invariants to memorize}',
    r'\section*{Book conventions}',
    r'\section*{Edition scope and source of truth}',
    r'\section*{Progressive exercise curve}',
]
hu_preface_order = [
    r'\section*{Alapfogalmak}',
    r'\section*{Hogyan olvasd a könyvet?}',
    r'\section*{Használd újra a Rust-tudásodat}',
    r'\section*{Minimum Rust a Velranhoz}',
    r'\section*{Velran 60 perc alatt}',
    r'\section*{Tíz megjegyzendő biztonsági invariáns}',
    r'\section*{A könyv konvenciói}',
    r'\section*{A kiadás hatóköre és a source of truth}',
    r'\section*{Fokozatos gyakorlási görbe}',
]
for edition, text, expected in [('EN', en_preface, en_preface_order), ('HU', hu_preface, hu_preface_order)]:
    pos = positions(text, expected)
    if any(p < 0 for p in pos) or pos != sorted(pos):
        errors.append(f'{edition} preface learning sections are missing or out of beginner-first order')

section_re = re.compile(r'^\\(?:sub)*section\*?\{', re.M)
for key in sorted(set(en) & set(hu)):
    en_shape = len(section_re.findall(en[key].read_text(encoding='utf-8')))
    hu_shape = len(section_re.findall(hu[key].read_text(encoding='utf-8')))
    if en_shape != hu_shape:
        errors.append(f'chapter {key}: section/subsection count differs: EN={en_shape}, HU={hu_shape}')

# 6c. Exercise-mode instructions live once in the preface; chapters keep only the short mode marker.
for key in sorted(set(en) & set(hu)):
    if key == '00':
        continue
    en_text = en[key].read_text(encoding='utf-8')
    hu_text = hu[key].read_text(encoding='utf-8')
    if r'\paragraph{Done when.}' in en_text:
        errors.append(f'{en[key]}: repeated generic Done when boilerplate; keep it in the preface')
    if r'\paragraph{Akkor kész.}' in hu_text:
        errors.append(f'{hu[key]}: repeated generic Akkor kész boilerplate; keep it in the preface')

# 7. The core web-development chapters must teach the same request lifecycle.
lifecycle_en = {
    '03': 'The request lifecycle: one mental model for most web work',
    '05': 'Apply the lifecycle to a CRUD mutation',
    '07': 'Where authentication and authorization sit in the lifecycle',
    '09': 'The same lifecycle also applies to APIs',
}
lifecycle_hu = {
    '03': 'A request életciklusa: egyetlen mentális modell a legtöbb webes feladathoz',
    '05': 'A request-életciklus alkalmazása CRUD módosításra',
    '07': 'Hol helyezkedik el az authentication és authorization az életciklusban?',
    '09': 'Ugyanez az életciklus érvényes az API-kra is',
}
for key, phrase in lifecycle_en.items():
    if phrase not in en[key].read_text(encoding='utf-8'):
        errors.append(f'{en[key]}: missing request-lifecycle teaching block')
for key, phrase in lifecycle_hu.items():
    if phrase not in hu[key].read_text(encoding='utf-8'):
        errors.append(f'{hu[key]}: missing request-lifecycle teaching block')

# 8. Core security invariants are deliberately repeated and paired across editions.
expected_invariant_chapters = {'03', '05', '06', '07', '08', '09', '13', '14', '17', '18'}
en_invariant_total = 0
hu_invariant_total = 0
for key in sorted(expected_invariant_chapters):
    en_text = en[key].read_text(encoding='utf-8')
    hu_text = hu[key].read_text(encoding='utf-8')
    en_count = en_text.count(r'\invariantblock{Security invariant:')
    hu_count = hu_text.count(r'\invariantblock{Biztonsági invariáns:')
    en_invariant_total += en_count
    hu_invariant_total += hu_count
    if en_count != 1:
        errors.append(f'{en[key]}: expected exactly one core Security invariant block, found {en_count}')
    if hu_count != 1:
        errors.append(f'{hu[key]}: expected exactly one core Biztonsági invariáns block, found {hu_count}')

if en_invariant_total != 10 or hu_invariant_total != 10:
    errors.append(f'core security invariant count differs from expected 10: EN={en_invariant_total}, HU={hu_invariant_total}')

for phrase in ['Ten security invariants to memorize', 'route has an explicit access policy', 'CORS is a browser-origin policy']:
    if phrase not in en_preface:
        errors.append(f'English preface missing core security-invariant guidance: {phrase}')
for phrase in ['Tíz megjegyzendő biztonsági invariáns', 'minden route explicit access policyt kap', 'a CORS böngészős origin-policy']:
    if phrase not in hu_preface:
        errors.append(f'Hungarian preface missing core security-invariant guidance: {phrase}')

# 9. High-value compiler/verifier diagnostics are taught where the concept first matters.
expected_diagnostic_chapters = {'03', '05', '06', '07', '08', '09', '13', '18'}
for key in sorted(expected_diagnostic_chapters):
    en_text = en[key].read_text(encoding='utf-8')
    hu_text = hu[key].read_text(encoding='utf-8')
    if en_text.count(r'\diagnosticblock{') != 1:
        errors.append(f'{en[key]}: expected exactly one diagnostic teaching block')
    if hu_text.count(r'\diagnosticblockhu{') != 1:
        errors.append(f'{hu[key]}: expected exactly one diagnostic teaching block')

expected_codes = {
    '03': 'SEC-A01-001', '05': 'SEC-A01-005', '06': 'SEC-A05-001',
    '07': 'SEC-A01-022', '08': 'SEC-FILE-001', '09': 'SEC-DATA-004',
    '13': 'SEC-EFFECT-002', '18': 'SEC-PROD-002',
}
for key, code in expected_codes.items():
    if code not in en[key].read_text(encoding='utf-8'):
        errors.append(f'{en[key]}: missing expected diagnostic code {code}')
    if code not in hu[key].read_text(encoding='utf-8'):
        errors.append(f'{hu[key]}: missing expected diagnostic code {code}')


# 10. Rust knowledge transfer is explicit in the chapters where beginners need it most.
expected_rust_bridge_chapters = {'01', '03', '04', '05', '06', '07', '09', '11'}
for key in sorted(expected_rust_bridge_chapters):
    en_text = en[key].read_text(encoding='utf-8')
    hu_text = hu[key].read_text(encoding='utf-8')
    if en_text.count(r'\rustbridgeblock{') != 1:
        errors.append(f'{en[key]}: expected exactly one Rust -> Velran bridge block')
    if hu_text.count(r'\rustbridgeblockhu{') != 1:
        errors.append(f'{hu[key]}: expected exactly one Rust -> Velran bridge block')

for phrase in ['Reuse your Rust knowledge', 'Advanced Rust topics such as unsafe code']:
    if phrase not in en_preface:
        errors.append(f'English preface missing Rust knowledge-transfer guidance: {phrase}')
for phrase in ['Használd újra a Rust-tudásodat', 'Az unsafe Rust']:
    if phrase not in hu_preface:
        errors.append(f'Hungarian preface missing Rust knowledge-transfer guidance: {phrase}')

# 11. The fast path has a bounded Minimum Rust prerequisite instead of a hidden full-Rust requirement.
for phrase in ['Minimum Rust for Velran', 'Ready-to-start test.', 'What you can postpone']:
    if phrase not in en_preface:
        errors.append(f'English preface missing Minimum Rust guidance: {phrase}')
for phrase in ['Minimum Rust a Velranhoz', 'Indulási önellenőrzés.', 'Mit halaszthatsz későbbre?']:
    if phrase not in hu_preface:
        errors.append(f'Hungarian preface missing Minimum Rust guidance: {phrase}')
if len(velran_blocks(en_dir / '00-preface.tex')) != len(velran_blocks(hu_dir / '00-eloszo.tex')):
    errors.append('preface Minimum Rust listing count differs between EN and HU')

# 12. The preface fast-start material stays unnumbered and paired across editions.
if r'\section*{Velran in 60 minutes}' not in en_preface:
    errors.append('English preface missing unnumbered Velran in 60 minutes section')
if r'\section*{Velran 60 perc alatt}' not in hu_preface:
    errors.append('Hungarian preface missing unnumbered Velran 60 perc alatt section')
if r'\section*{Progressive exercise curve}' not in en_preface or r'\section{Progressive exercise curve}' in en_preface:
    errors.append('English progressive exercise curve must be an unnumbered preface section')
if r'\section*{Fokozatos gyakorlási görbe}' not in hu_preface or r'\section{Fokozatos gyakorlási görbe}' in hu_preface:
    errors.append('Hungarian progressive exercise curve must be an unnumbered preface section')
for phrase in ['Page and route', 'Authentication and authorization', 'velran-cli check']:
    if phrase not in en_preface:
        errors.append(f'English 60-minute path missing teaching step: {phrase}')
for phrase in ['Page és route', 'Authentication és authorization', 'velran-cli check']:
    if phrase not in hu_preface:
        errors.append(f'Hungarian 60-minute path missing teaching step: {phrase}')

if errors:
    for e in errors:
        print(f'book example verification: {e}', file=sys.stderr)
    raise SystemExit(1)

print('book Velran example verification passed')
PY
