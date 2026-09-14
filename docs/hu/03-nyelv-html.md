<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 3. Nyelvi és HTML alapok

## Statementhatárok

A Velran explicit pontosvesszőt használ. Az egyszerű statementek `;` jellel záródnak, a sortörés csak whitespace. A blokk nélküli top-level deklarációk (`mod ...;`, `route ... => handler;`) szintén pontosvesszősök; a `{ ... }` blokkal záródó deklarációk és control-flow blokkok után nincs `;`.

```vrn
mod products;
route product GET "/products/:id<i64>" public => products::show;

let gross = price * quantity;
return Ok(json(gross));
```

Részletesen: [Statement terminátorok](55-statement-terminatorok.md).

## Kifejezések, aritmetika és logika

A nyelv támogatja az ellenőrzött `+`, `-`, `*`, `/`, `%` aritmetikát; `i64` esetén `<<`, `>>`, `&`, `^`, `|` operátorokat; valamint `bool` értékekre `!`, `&&`, `||` logikát. A `&&` és `||` short-circuit módon működik. Az `f32` matematika Rust-szerű metódusokat használ, többek között `.ln()`, `.log10()`, `.log()`, `.exp()`, `.powf()`, `.round()`, `.floor()` és `.ceil()`.

```vrn
let bucket = id % 16;
let flags = mask | 4;
let visible = published && !deleted;
let rounded = score.round();
```

Részletesen: [Numerikus operátorok, f32 matematika és monoton időmérés](43-matematika-es-idomeres.md).

## String műveletek

A Unicode-tudatos String API tartalmazza a trimminget és case conversiont, keresést, `replace`/`split` műveleteket, substring/index műveleteket, karakterelérést és ismétlést.

```vrn
let cleaned = title.trim();
let slugText = cleaned.to_lowercase().replace(" ", "-");
let prefix = substring(slugText, 0, 8);
let found = indexOf(slugText, "vrn");
```

Részletesen: [String műveletek](45-string-muveletek.md) és [Reguláris kifejezések](48-regexp.md).

## Model

```vrn
model Product {
    id: i64
    name: String
    price: i64
}
```

A model mezőtípusok nem korlátozódnak a korai `String`/`i64`/`bool` készletre; a nyelv támogatott üzleti és domain típusait a kapcsolódó típusfejezetek dokumentálják.

## Page és action

```text
#[page]
fn product(ctx: PageContext, db: Db, id: i64) -> Result<Html, PageError> {
    ...
}
```

```text
#[action]
fn create(ctx: ActionContext, db: Db, name: String, price: i64)
    -> Result<Redirect, PageError> {
    ...
}
```

## HTML

```text
return Ok(html {
    <h1>{{ product.name }}</h1>
})
```

A `{{ ... }}` HTML-escaped. DB-ből érkező stringre is ugyanaz a szabály.

Lista:

```text
@for product in products {
    <li>{{ product.name }}</li>
}
```

Optional model:

```text
@if product {
    <h1>{{ product.name }}</h1>
}
```

Typed URL:

```text
<a @href(product, product.id)>View</a>
<form method="post" @action(delete, product.id)>
```

Ne használj dinamikus `href="{{ value }}"` mintát; URL csak typed route helperen menjen.
