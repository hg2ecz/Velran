<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Velran editor syntax highlighting

## Vim

Copy:

- `vim/syntax/velran.vim` to `~/.vim/syntax/velran.vim`
- `vim/ftdetect/velran.vim` to `~/.vim/ftdetect/velran.vim`

Open any `*.vrn` file; `:set filetype?` should report `velran`.

## Midnight Commander / mcedit

Copy `mcedit/velran.syntax` to:

- `~/.local/share/mc/syntax/velran.syntax`

Then add the Velran entry to `~/.local/share/mc/syntax/Syntax`:

```
file .\*\\.vrn$ Velran
include velran.syntax
```

The first matching `file` entry wins, so place the Velran entry before any generic
catch-all rule. System-wide MC syntax files are normally under the installation's
`share/mc/syntax/` directory; the user directory above overrides them.

## Maintenance

The syntax files track the public Velran source language only. Compiler internals such as
verified IR, bytecode, register VM instructions, cache artifacts, and backend selection are
implementation details and intentionally are not highlighted as Velran syntax.

## Validation

The repository editor definitions are checked against the current public Velran surface.
The Vim syntax file is load-tested with Vim when Vim is available, and both definitions
track `.vrn`, Rust-surface module/visibility constructs (`pub`, `use`, `struct`, `impl`,
`self`, `Self`, `crate`, `super`), sum values, and Velran route/security keywords.
