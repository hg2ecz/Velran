# Velran source-level common modules

These files are ordinary Velran source, not compiler/runtime plugins. Copy the module next to the application source that declares it, or vendor it into the application's source tree.

## `commonmark.vrn`

A bounded, security-first CommonMark-oriented renderer written in Velran. Include it from `main.vrn`:

```vrn
mod commonmark;
```

Then render Markdown to typed `SafeHtml`:

```vrn
let rendered = commonmark::render(&source);
```

The Velran engine intentionally contains no Markdown parser or Markdown-specific template directive. It exposes only generic SafeHtml construction primitives with escaping, tag allowlisting and URL-policy enforcement.

## Language/runtime boundary

`commonmark.vrn` intentionally exercises ordinary Velran language features rather than receiving Markdown privileges from the engine. Its parser uses verified pure helpers, scalar value parameters, helper calls with expression arguments, Rust-like `if`/`else if`/`else`, bounded recursion/resource accounting and normal string operations.

`@markdown(...)` is not a template directive. Unknown call-shaped template directives are compile errors, so a misspelled engine feature cannot silently render as text. The renderer produces `SafeHtml` only through the generic typed HTML builders.

Raw HTML pass-through is intentionally disabled. The module therefore implements a security-hardened CommonMark profile rather than every permissive HTML rule from upstream CommonMark.
