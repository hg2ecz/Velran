<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Bounded JSON boundary

JSON is framework-owned input/output infrastructure. Application shards do not parse raw HTTP JSON bytes themselves.

The request path is:

```text
HTTP body -> bounded framework JSON parser -> route/type validation -> typed native inputs -> cdylib
```

The parser rejects duplicate object keys, NUL-containing strings, out-of-range integers, non-finite numbers, excessive nesting, oversized strings, excessive array items, and excessive object fields before route binding.

Global defaults:

```toml
[limits]
max_body_bytes = 262144
max_json_depth = 32
max_json_string_bytes = 65536
max_json_array_items = 4096
max_json_object_fields = 1024
```

The same JSON limits may be tightened per domain. CLI overrides are also available:

```text
--max-json-depth
--max-json-string-bytes
--max-json-array-items
--max-json-object-fields
```

The current stable route surface binds a declared JSON object directly to typed handler parameters. Example: `examples/json-framework-boundary/app.vrn`.

The next language step is first-class Rust-like `Json<T>` input/output syntax. That syntax must compile to the same bounded framework parser and typed native boundary; it must not add a second JSON parser inside generated cdylibs.
