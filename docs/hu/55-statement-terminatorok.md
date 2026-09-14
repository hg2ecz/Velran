<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Statement terminátorok

A Velran minden egyszerű statementet explicit pontosvesszővel (`;`) zár. A sortörés csak whitespace; nincs automatikus semicolon insertion.

```vrn
let total = price
    * quantity
    + shipping;
retries = retries + 1;
authorize article owner authorUsername or role Publisher;
flash success "Saved";
return Ok(json(total));
```

A `transaction db { ... }` blokkon belüli önálló mutating query-hívások és az `audit ...` statementek szintén `;` jelet igényelnek.

```vrn
transaction db {
    updateArticle(tx, id, title)?;
    audit Article id action update from oldTitle to title;
}
```

A blokkokat a `}` zárja, utánuk nincs pontosvessző:

```vrn
if ready {
    state = 1;
}
while state < 3 {
    state = state + 1;
}
```

Ugyanez érvényes a blokkos deklarációkra (`model`, valamint az attribútumos Rust-szerű `#[page] fn`, `#[action] fn`, `#[query] fn` függvények). A `mod path;` és a `route ... => handler;` pontosvesszős, mert blokk nélküli deklaráció. A route több sorba tördelhető, de csak a záró `;` terminálja.

## Top-level, blokk nélküli deklarációk

A `mod` és `route` deklaráció is explicit `;` jellel záródik:

```vrn
mod catalog::pages;

route catalogIndex GET "/catalog"
    query page<i64>
    validate page range 1 1000
    public => catalog::pages::index;
```

A route scanner csak a záró `;` után tekinti teljesnek a deklarációt; a sortörés és az `=> handler` önmagában nem terminátor.
