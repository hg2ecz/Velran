<!-- VELRAN-DOC-STATUS: 2026-09-15 -->
> **Documentation status (2026-09-15):** Current language contract for verified pure computation. Security comes first; productivity is expanded only where the compiler can preserve type, authority, aliasing and resource invariants.

# Verified pure functions

Verified pure functions are Velran's reusable application-compute layer. They are ordinary Rust-like `fn` declarations that run without ambient web/server authority and are lowered through verified IR to generated safe Rust.

They are intended for parsers, renderers, validation helpers, transforms, numeric kernels and other reusable computation that should not need direct database, filesystem, network, process, environment, thread, FFI or `unsafe` access.

## Supported parameter families

The scalar/borrowed family supports:

```text
i64
bool
&str
&[String]
&Struct
```

`i64` and `bool` are immutable by-value parameters. Borrowed strings, string lists and structs remain explicit immutable borrows.

The mutable numeric hot-path family supports fixed arrays:

```text
&mut [f32; N]
```

with `N` in the compiler-verified fixed-array range. Mutable numeric parameters use an explicit `&mut` call-site borrow. The scalar/borrowed family and mutable numeric family are deliberately not mixed in one pure function while they use separate verified internal ABIs.

Example:

```velran
fn choose(value: i64, enabled: bool) -> i64 {
    if enabled {
        return value;
    }
    return 0;
}

fn normalize(input: &str) -> String {
    return input.trim().to_lowercase();
}

fn kernel(real: &mut [f32; 16]) -> f32 {
    real[0] = real[0] + 1.0f32;
    return real[0];
}
```

A mutable array call must remain explicit:

```velran
kernel(&mut real)
```

The compiler does not silently turn `kernel(real)` into a mutable borrow.

## Returns

Verified pure functions support the currently verified safe return surface, including scalar values, owned strings/string lists, `SafeHtml`, declared structs and the supported `Option<T>` / `Result<T,E>` forms.

Nested returns inside verified `if`/`else`/`else if`/`while` blocks are type-checked against the declared function return type. The current lowering still requires a final top-level return for non-unit functions; early nested returns do not remove that final-return contract.

## Pure-to-pure calls

Scalar/borrowed pure functions may call other scalar/borrowed pure helpers. Arguments are normal typed expressions rather than variable names only:

```velran
fn step(value: i64, enabled: bool) -> i64 {
    if enabled {
        return value + 1;
    }
    return value;
}

fn run(value: i64) -> i64 {
    let first = step(value + 1, true);
    let mut out = 0;
    out = step(first, false);
    return out;
}
```

Calls continue to pass through the verified pure-call path. Child work is charged to the same instruction/allocation accounting rather than becoming an unmetered escape hatch.

The scalar and mutable-numeric helper families may not call across their internal ABI boundary. Keep scalar orchestration and mutable numeric kernels separate until a unified verified internal call ABI exists.

## Recursion and resource safety

Scalar/borrowed pure recursion is supported but runtime depth-bounded. Generated code carries a pure-call recursion budget with a current hard maximum of 64 nested calls. Instruction and allocation budgets still apply, so recursion is bounded by both depth and normal request/resource accounting.

Recursive cycles that touch the mutable numeric hot-path family are rejected at compile time. This keeps the special mutable-array ABI fail-closed against unbounded recursive stack growth.

The recursion-depth constant is an implementation hard limit, not a promise that applications should routinely consume the entire depth. Prefer iterative code where it is clearer and cheaper.

## Control flow

Verified pure computation supports normal Rust-like branching:

```velran
if condition {
    ...
} else if other_condition {
    ...
} else {
    ...
}
```

Each condition is evaluated exactly once. The compiler preserves the condition's static security metadata when it lowers branching through internal temporaries; an untrusted boolean is not silently reclassified as trusted.

`while` remains resource-budgeted. Branches and loops do not grant capabilities that the surrounding pure function does not have.

## Strings and immutable borrows

Rust-like string methods such as `.trim()`, `.to_lowercase()` and `.to_string()` are verified string operations. String-producing operations remain allocation-accounted.

For string-oriented builtins such as `substring(...)` and `charAt(...)`, an explicit immutable borrow is accepted where the builtin's argument contract is string-like:

```velran
let tail = substring(&text, 1);
let first = charAt(&tail, 0);
```

The compiler does not erase `&` generically. A builtin argument that does not accept an immutable borrow remains a compile error. `&mut` is never accepted as a substitute for an immutable string argument.

## Template and SafeHtml boundary

`SafeHtml` is a typed output boundary, not a trusted-string convention. Ordinary `String` interpolation into `html { ... }` is escaped; only a `SafeHtml` value is emitted without another escaping pass.

The compiler/framework owns the generic SafeHtml constructors, escaping, tag allowlist and URL policy. Application libraries own domain-specific rendering logic. Markdown/CommonMark is therefore implemented in Velran source (`common/commonmark.vrn`), not as an engine feature.

There is intentionally no `@markdown(...)` engine directive. Unknown template call-directives shaped like `@name(...)` fail fast instead of silently becoming text, which catches misspellings and prevents accidental pseudo-features.

## Diagnostics

Frontend expression/control-flow diagnostics preserve source location. File compilation reports the source path and line for these errors; nested `if`/`else`/`while` parsing carries the original line base instead of losing location information.

Diagnostics should identify the actual language contract that failed. For example, a legacy syntax error is reported as legacy syntax rather than being misreported as a borrow error.

## Security invariants

Verified pure-function productivity must not weaken these rules:

- no ambient filesystem/network/process/environment/thread/FFI/unsafe authority;
- mutable borrows stay explicit;
- scalar/borrowed and mutable numeric helper ABIs remain separated;
- pure calls inherit and charge resource budgets;
- scalar recursion has a hard depth bound;
- numeric-hot-path recursive cycles fail closed;
- expression and return types are checked across nested control flow;
- string allocations are accounted;
- `SafeHtml` remains a typed XSS boundary, not a string flag.

The design rule is: **security first, productivity second — but do not force application code into unnatural rewrites when a normal construct can be implemented without weakening the invariants above.**
