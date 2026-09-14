<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Regular expressions

Velran provides a small, resource-bounded regular-expression API for request processing and text normalization.

```vrn
let ok = regexMatch(text, "^[A-Z]{2}-[0-9]{4}$");
let normalized = regexReplace(text, "[^A-Za-z0-9]+", "-");
let captures = regexCaptures(text, "^(?P<name>[a-z]+)-(?P<id>[0-9]+)$");
```

The functions are strictly typed:

```text
regexMatch(String, String) -> bool
regexReplace(String, String, String) -> String
regexCaptures(String, String) -> BTreeMap<String,String>
```

`regexCaptures` returns an empty dictionary when there is no match. Successful captures are available by numeric keys (`"0"`, `"1"`, ...) and named capture keys. Optional unmatched groups are omitted. Use `containsKey` before indexing when a capture is optional.

## Safety and limits

The runtime uses Rust's `regex` engine, whose supported syntax avoids backreferences and look-around constructs that require unbounded backtracking. Velran adds explicit limits:

- pattern: at most 4096 UTF-8 bytes;
- input: at most 1 MiB;
- replacement template: at most 16 KiB;
- at most 64 capture groups;
- generated replacement/capture data: at most 16 MiB and still charged to the normal runtime allocation budget.

Invalid patterns and limit violations are request errors; they do not panic the worker. Regex operations also have a higher instruction cost than simple scalar operations.

The API deliberately keeps compiled regex objects out of the language value model. They are implementation details, which keeps application state deterministic and leaves room for a bounded compiled-pattern cache later without changing Velran source semantics.
