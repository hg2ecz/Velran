# Source-level CommonMark renderer

`commonmark.vrn` is a CommonMark-oriented Markdown renderer written in Velran itself. It is not part of the compiler/runtime engine: there is no Markdown-specific engine feature.

Copy `commonmark.vrn` next to your application's `main.vrn`, then include it with:

```vrn
mod commonmark;
```

Render with:

```vrn
let rendered = commonmark::render(&source);
```

A `SafeHtml` value may be interpolated in an `html { ... }` template. Ordinary `String` values remain HTML-escaped.

## Velran language role

This example is also a general-purpose Velran language stress test, not a framework exception. The renderer uses verified pure helper composition, by-value scalar parameters, explicit immutable borrows, Rust-like `if`/`else if`/`else`, bounded scalar recursion and allocation-accounted strings. None of those capabilities is Markdown-specific.

Unknown `@name(...)` template call-directives fail at compile time. There is no Markdown-specific engine directive; the only engine-owned privilege here is the generic typed `SafeHtml` boundary.

## Security profile

The module follows a deliberately safe CommonMark profile:

- raw HTML in Markdown is rendered as text rather than passed through;
- all ordinary text passes through `safeHtmlText`;
- output elements pass through the engine's small fixed `SafeHtml` tag allowlist;
- links allow only relative/fragment, `https`, `http`, and `mailto` targets;
- line splitting is bounded to 2048 lines;
- the application gets no arbitrary raw-HTML constructor.

The source module covers the application-oriented CommonMark core with explicit block and inline parsing: ATX and Setext headings, thematic breaks, indented and variable-length fenced code blocks, bounded nested blockquotes, grouped unordered/ordered lists, multi-line paragraphs, soft and hard line breaks, arbitrary-length backtick code spans, strong/emphasis, balanced-parenthesis inline links, URI/email autolinks, and punctuation-only backslash escaping. CRLF/CR input is normalized before block parsing.

Raw HTML blocks/inlines are intentionally disabled. Images, reference-link definitions, full loose/nested-list semantics, entity decoding, and ordered-list `start` attributes are not emitted by this security profile yet. The module is therefore a security-hardened CommonMark profile, not a byte-for-byte implementation of every upstream rendering rule.
