<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# String builtins

Velran provides a typed, Unicode-aware string-compute core for application logic. These operations are lowered into bounded generated Rust and retain instruction/allocation accounting; there is no bytecode runtime fallback.

## API

```vrn
let cleaned = text.trim();
let left = text.trim_start();
let right = text.trim_end();
let low = cleaned.to_lowercase();
let high = cleaned.to_uppercase();
let n = cleaned.chars().count();
let has = low.contains("rust");
let prefix = low.starts_with("vrn");
let suffix = low.ends_with("lang");
let rewritten = low.replace(" ", "-");
let part = substring(cleaned, 1, 3);
let first = indexOf(cleaned, "vrn");
let last = lastIndexOf(cleaned, "a");
let ch = charAt("Velran", 1);
let repeated = "vrn".repeat(3);
let pieces = splitBounded(cleaned, ",", 4096);
```

Signatures:

```text
String::chars().count() -> i64
String::trim() -> String
String::trim_start() -> String
String::trim_end() -> String
String::to_lowercase() -> String
String::to_uppercase() -> String
String::to_string() -> String
String::contains(String) -> bool
String::starts_with(String) -> bool
String::ends_with(String) -> bool
String::replace(String, String) -> String
splitBounded(String, String, i64) -> Vec<String>
substring(String, i64) -> String
substring(String, i64, i64) -> String
indexOf(String, String) -> i64
lastIndexOf(String, String) -> i64
charAt(String, i64) -> String
String::repeat(i64) -> String
```

`.chars().count()`, `substring`, `indexOf`, `lastIndexOf`, and `charAt` use Unicode scalar-value positions rather than UTF-8 byte offsets. `indexOf` and `lastIndexOf` return `-1` when no match exists. `substring(text, start)` returns the suffix from `start`; the three-argument form takes a character count and clips the end to the available string length. Invalid negative/out-of-range indices fail closed.

Case conversion uses Unicode-aware Rust string conversion. `replace` rejects an empty search string, and `split` rejects an empty delimiter. A split is capped at 4096 result items. `repeat` rejects negative counts and is charged against the request allocation budget before allocation.

## Explicit immutable borrows at string builtin boundaries

String-oriented builtins may receive an explicit immutable borrow when the corresponding argument contract is string-like. This keeps Rust-like borrowed call sites usable in verified pure libraries:

```velran
let tail = substring(&text, 1);
let ch = charAt(&tail, 0);
```

The compiler does **not** strip `&` generically. A non-string builtin argument that is explicitly borrowed remains a compile error, and `&mut` is not accepted for immutable string arguments. This preserves the borrow boundary instead of turning it into an implicit coercion rule.

`.to_string()` is a verified owned-string operation and is allocation-accounted like other string-producing operations.

## Resource accounting

String-producing builtins reserve output memory against the request and resource-scope allocation budgets before producing the result. Case conversion is conservatively charged for possible Unicode expansion. `replace`, `split`, and `repeat` calculate or conservatively estimate output size before allocation.

The builtins also have explicit instruction costs. They remain convenient application operations, not an unmetered path around the runtime budget.

See `examples/string-operations/main.vrn` and `examples/string-list/main.vrn` for runnable examples.

## Statement syntax

The examples use explicit `;` terminators because each `let` is a simple statement. A newline does not terminate a statement. See [Statement terminators](56-statement-terminators.md). `String + String` concatenation and the numeric/logical operator precedence are documented in [Numeric operators, f32 math and monotonic timing](44-math-and-timing.md).

## Explicitly bounded split

Use `splitBounded(text, delimiter, maxItems)` when the domain has a smaller known cardinality limit. `maxItems` must be a compile-time integer literal in `1..4096`, and execution fails rather than truncating when the input would exceed the bound.
