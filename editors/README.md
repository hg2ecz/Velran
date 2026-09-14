<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Velran editor syntax highlighting

## Vim

Copy:

- `vim/syntax/velran.vim` to `~/.vim/syntax/velran.vim`
- `vim/ftdetect/velran.vim` to `~/.vim/ftdetect/velran.vim`

Open any `*.vrn` file; `:set filetype?` should report `velran`.

## Midnight Commander / mcedit

MC syntax registration differs slightly by distribution. Copy `mcedit/velran.syntax` into the user MC syntax directory and add an entry for `*.vrn` to the local `Syntax` file, for example:

```
file \\.vrn$ Velran
include velran.syntax
```

Typical user locations are under `~/.local/share/mc/mcedit/` or `~/.config/mc/`; use the location used by your installed MC package.

## Maintenance

The syntax files track the public Velran source language only. Compiler internals such as
verified IR, bytecode, register VM instructions, cache artifacts, and backend selection are
implementation details and intentionally are not highlighted as Velran syntax.
