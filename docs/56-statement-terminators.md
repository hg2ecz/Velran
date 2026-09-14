<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Statement terminators

Velran uses an explicit semicolon (`;`) to terminate every simple statement. A newline is whitespace only; there is no automatic semicolon insertion.

```vrn
let total = price
    * quantity
    + shipping;
retries = retries + 1;
authorize article owner authorUsername or role Publisher;
flash success "Saved";
return Ok(json(total));
```

Inside `transaction db { ... }`, standalone mutating query calls and `audit ...` statements also require `;`.

```vrn
transaction db {
    updateArticle(tx, id, title)?;
    audit Article id action update from oldTitle to title;
}
```

Blocks are closed by `}` and do not take a trailing semicolon:

```vrn
if ready {
    state = 1;
}
while state < 3 {
    state = state + 1;
}
```

The same principle applies to block declarations such as `model` and attributed Rust-like functions (`#[page] fn`, `#[action] fn`, `#[query] fn`): no `;` follows the closing brace. `mod path;` and `route ... => handler;` are semicolon-terminated because they are non-block declarations. A route may span multiple lines, but only the final `;` terminates it.

## Non-block top-level declarations

`mod` and `route` declarations also use an explicit `;`:

```vrn
mod catalog::pages;

route catalogIndex GET "/catalog"
    query page<i64>
    validate page range 1 1000
    public => catalog::pages::index;
```

The route scanner considers the declaration complete only after the final `;`; neither a newline nor `=> handler` alone terminates a route.
