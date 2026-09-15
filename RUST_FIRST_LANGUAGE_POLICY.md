<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Rust-first Velran language policy

Velran uses Rust syntax as the default application language. A separate Velran spelling is permitted only when it carries security, resource, lifecycle, or web-framework semantics that plain Rust syntax does not express safely enough for the platform.

## Rule

1. If a Rust construct is deterministic, local, verifiable, and compatible with the application sandbox, accept the Rust spelling.
2. Lower safe/local operations to generated safe Rust inside the application cdylib. Do not call the host runtime for ordinary arithmetic, strings, collections, branches, loops, formatting, or local HTML construction.
3. Reject ambient-authority Rust APIs such as direct filesystem, network, process, environment, thread, arbitrary FFI, raw pointers, and `unsafe`. Those effects are available only through explicit framework capabilities and verified host ABI operations.
4. Keep framework syntax only for routes/pages/actions, authorization and capabilities, resource limits, critical/idempotent operations, request contracts, database/storage/outbound effects, uploads, and other security-relevant web semantics.
5. Prefer bounded framework operations when the standard Rust operation would make resource use unbounded or unverifiable.

## Current Rust-first surface

Examples include `fn`, `let`, `let mut`, Rust primitive/container type spellings, `vec![x; n]`, `.len()`, `as f32`, string methods, `BTreeMap::new()`, `.contains_key()`, numeric methods such as `.sin()`, `.cos()`, `.sqrt()`, `.powf()`, and elapsed timing written as `std::time::Instant::now()` plus `started.elapsed().as_nanos()`.

The compiler may lower some of these forms to compact verified IR rather than forwarding source text verbatim. The user-visible contract is Rust syntax; verification remains authoritative before code generation.

## Verified pure compute contract

The Rust-first policy applies to verified pure functions as well: normal local constructs should use normal Rust-like spellings when the compiler can verify them without widening authority.

Current pure computation supports by-value `i64` and `bool`, immutable `&str`, `&[String]` and borrowed declared structs, plus a separate explicit `&mut [f32; N]` numeric-kernel family. Scalar/borrowed helpers may call one another with typed expression arguments. Scalar recursion is runtime depth-bounded and resource-accounted; recursive cycles that touch the mutable numeric hot path are rejected. The scalar and numeric helper families cannot cross-call while their verified internal ABIs remain separate.

`if` / `else if` / `else` is first-class verified control flow. Lowering evaluates each condition once and preserves its static security metadata. String `.to_string()` and explicit immutable borrows at string-oriented builtin positions are verified operations rather than generic source-text passthrough.

See [`docs/58-verified-pure-functions.md`](docs/58-verified-pure-functions.md) for the normative language contract.

## Security boundary

The generated application shard is std-only and safe Rust. The verifier, not `rustc` alone, decides whether an operation is allowed in the web sandbox. Pure code receives no HostApi capability. External effects receive only the explicitly verified capability surface required by the handler.

## rustc-first compiler boundary

Velran must not grow a second Rust compiler. The platform owns only the checks that are security- or resource-relevant to the web sandbox; ordinary Rust syntax/type errors belong to the selected trusted `rustc` toolchain.

The native build therefore follows this order:

1. normalize the supported Rust-like surface;
2. reject capability bypasses and unsafe/ambient-authority Rust constructs before native compilation (`SEC-RUST-001`);
3. run Velran web/security/resource verification and produce verified executable IR;
4. generate safe, std-only Rust with `#![forbid(unsafe_code)]` in the user-derived implementation module;
5. invoke one isolated `rustc` process for the cdylib;
6. surface the real bounded `rustc` diagnostics instead of re-implementing its type checker.

The direct rustc invocation removes Cargo/Rust injection points such as `RUSTFLAGS`, compiler wrappers, Cargo home and encoded workspace flags. Application-generated code receives no `--extern` dependency graph. The compiler-owned ABI shim remains the only trusted unsafe boundary.

### Diagnostics policy

A failed rustc invocation is captured as a first-class build diagnostic (bounded to prevent log/memory amplification). It is available to the console and error chain. Frontend expression/control-flow diagnostics also preserve source location; file compilation reports the originating path and line, including nested pure `if`/`else`/`while` parsing. With non-production `debug_compile_errors`, source reload also writes the detailed diagnostic to the server error log and serves an HTML-escaped, domain-scoped developer error page while the last known-good generation remains active. Production policy continues to reject detailed web compiler diagnostics and rustc reproduction artifacts.

### Inherent `impl` status

Inherent `impl Type { ... }` blocks are supported only through the verified pure-compute path. The compiler registers method symbols, rewrites an immutable `&self` receiver to an internal borrowed struct parameter, and feeds the method body through the same type/resource/security lowering used by ordinary pure functions. Raw user `impl` bodies are never pasted into generated Rust. Owned `self`, `&mut self`, generic impls and trait impls remain fail-closed until their security and resource semantics are defined. Ordinary Rust typing of the generated safe code remains rustc's responsibility.

### Library visibility and clean API boundaries

Reusable Rust-like library items are private by default. `pub` is accepted for structs, enums, ordinary pure functions, inherent methods and struct fields when they intentionally form a cross-module API. A public pure function may not expose a private struct in its supported parameter/return surface. `impl` blocks themselves cannot be `pub`; expose only the methods that belong to the API.

This is an encapsulation boundary, not a web authority boundary. Pages, actions, queries and routes continue to use compiler-owned route/capability/security policy. Nested module boundaries support `pub mod`; explicit `use path as alias;` namespace aliases are supported. Package ownership now permits one deliberately narrow re-export: `pub use path as alias;` at the package root, and only when `path` names an already-public module namespace in the same package. Wildcards, direct item re-exports, private modules and private transitive dependencies remain rejected. This is still an encapsulation convenience, never a web-authority grant.

Clean-code rule: keep implementation details private, prefer the smallest public surface, and do not duplicate visibility parsing or access checks across feature parsers; the compiler uses one shared visibility model and one access policy.

## Local library boundary
Local Velran libraries are explicit manifest-declared path dependencies. The compiler does not fetch or execute registry/git/build-script dependencies. Libraries are private-by-default computation/API packages and cannot introduce web/server authority; those declarations remain application-owned and pass the normal security verifier before rustc receives generated safe Rust.
