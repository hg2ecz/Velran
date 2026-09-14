#!/bin/sh
set -eu

python3 - <<'PY'
from pathlib import Path
import re
import sys

root = Path('.')
base = root / 'examples/secure-notes'
errors = []
required = [
    'main.vrn', 'models.vrn', 'queries.vrn', 'pages.vrn', 'actions.vrn',
    'migrations/0001_init.sql', 'README.md',
]
for rel in required:
    if not (base / rel).is_file():
        errors.append(f'missing {base / rel}')

def text(rel):
    return (base / rel).read_text(encoding='utf-8')

if not errors:
    main = text('main.vrn')
    pages = text('pages.vrn')
    actions = text('actions.vrn')
    queries = text('queries.vrn')
    models = text('models.vrn')
    migration = text('migrations/0001_init.sql')

    for module in ('models', 'queries', 'pages', 'actions'):
        if f'mod {module};' not in main:
            errors.append(f'main.vrn does not declare module {module}')

    routes = re.findall(r'(?ms)^\s*route\s+.*?;', pages + '\n' + actions)
    if len(routes) != 5:
        errors.append(f'expected 5 Secure Notes routes, found {len(routes)}')
    for route in routes:
        flat = ' '.join(route.split())
        if ' auth user ' not in f' {flat} ':
            errors.append(f'route is not explicitly auth user: {flat}')
        if ' public ' in f' {flat} ':
            errors.append(f'canonical Secure Notes route became public: {flat}')

    normalized_queries = ' '.join(queries.split())
    if 'WHERE ownerUsername = :ownerUsername' not in normalized_queries:
        errors.append('listNotes no longer scopes rows by ownerUsername')
    if 'queries::listNotes(db, authPrincipal)?' not in pages:
        errors.append('notes page no longer passes authPrincipal into the scoped list query')

    ownership_checks = (pages + '\n' + actions).count('authorize note owner ownerUsername;')
    if ownership_checks < 3:
        errors.append(f'expected at least 3 explicit note ownership checks, found {ownership_checks}')

    for bound in ('validate title length 1 120', 'body length 1 4000'):
        if bound not in actions:
            errors.append(f'missing request-bound validation: {bound}')

    for expected in (
        'enum NoteStatus', 'Draft', 'Published', 'createdAt: DateTime', 'updatedAt: DateTime'
    ):
        if expected not in models:
            errors.append(f'model contract missing {expected!r}')

    for expected in (
        "CHECK (length(title) BETWEEN 1 AND 120)",
        "CHECK (length(body) BETWEEN 1 AND 4000)",
        "CHECK (status IN ('Draft', 'Published'))",
        'CREATE INDEX notes_owner_id_idx ON notes(ownerUsername, id DESC)',
    ):
        if expected not in migration:
            errors.append(f'baseline migration missing invariant: {expected}')

    verify = (root / 'verify.sh').read_text(encoding='utf-8')
    if 'velran-cli -- check examples/secure-notes/main.vrn' not in verify:
        errors.append('verify.sh does not compiler-check examples/secure-notes/main.vrn')

    checkpoint_manifest = base / 'CHECKPOINTS.tsv'
    expected_checkpoints = {
        '01': '01-baseline',
        '05': '05-crud',
        '07': '07-auth',
        '09': '09-json-api',
        '12': '12-publication',
        '18': '18-production',
        '23': '23-release-readiness',
    }
    if not checkpoint_manifest.is_file():
        errors.append('missing Secure Notes CHECKPOINTS.tsv')
    else:
        rows = []
        for raw in checkpoint_manifest.read_text(encoding='utf-8').splitlines():
            if not raw or raw.startswith('#'):
                continue
            parts = raw.split('\t')
            if len(parts) != 4:
                errors.append(f'invalid checkpoint manifest row: {raw}')
                continue
            rows.append(parts)
        seen = {}
        for chapter, slug, entry, companions in rows:
            seen[chapter] = slug
            cp = root / entry
            if not cp.is_file():
                errors.append(f'checkpoint compiler entry missing: {entry}')
                continue
            cpdir = cp.parent
            for rel in ('main.vrn','models.vrn','queries.vrn','pages.vrn','actions.vrn','migrations/0001_init.sql','README.md'):
                if not (cpdir / rel).is_file():
                    errors.append(f'checkpoint {slug} missing {rel}')
            for rel in ('models.vrn','queries.vrn','pages.vrn','actions.vrn','migrations/0001_init.sql'):
                if (cpdir / rel).is_file() and (base / rel).is_file():
                    if (cpdir / rel).read_bytes() != (base / rel).read_bytes():
                        errors.append(f'checkpoint {slug} drifted from canonical baseline in {rel}; evolve intentionally and update the guard before accepting drift')
            verify_line = f'velran-cli -- check {entry}'
            if verify_line not in verify:
                errors.append(f'verify.sh does not compiler-check checkpoint {entry}')
            for companion in filter(None, companions.split(';')):
                if not (root / companion).is_file():
                    errors.append(f'checkpoint {slug} companion missing: {companion}')
        if seen != expected_checkpoints:
            errors.append(f'checkpoint chapter map mismatch: {seen!r}')

if errors:
    for error in errors:
        print(f'secure-notes verification: {error}', file=sys.stderr)
    raise SystemExit(1)
print('canonical Secure Notes verification passed')
PY
