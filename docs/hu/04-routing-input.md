<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 4. Routing, input és validáció

## Path

```text
route product GET "/products/:id<i64>" public => product;
```

A handlerben ugyanaz a név/típus kell:

```text
#[page]
fn product(ctx: PageContext, db: Db, id: i64) ...
```

## Query

```text
route products GET "/products"
    query page<i64> pageSize<i64>
    validate page range 1 100000 pageSize range 1 100
    public => products;
```

A schema closed-world: hiányzó, duplikált vagy ismeretlen mező `400`.

## Form

```text
route create POST "/products"
    form name<String> price<i64>
    validate name length 1 100 price range 0 100000000
    public => create;
```

Jelenlegi validation szabályok:

```text
length MIN MAX   // String
range MIN MAX    // i64
```

State-changing route-ot `#[action] fn` kezeljen. A kliens által küldött `price`, `role`, `tenantId` stb. csak input; authority-t mindig a szerver/DB ad.

## Keresőbarát path: Slug

```velran
route articleShow GET "/cikk/:slug<Slug>" public => articleShow;
```

A `Slug` canonical, max. 160 byte, és nem tetszőleges String. Így `../`, slash, szóköz és nem canonical forma nem jut át typed path paraméterként.

Új slug készítése:

```velran
let articleSlug = slug(title);
```

Részletesen: [Projektstruktúra, modulok és keresőbarát URL-ek](21-modules-slugs-project-layout.md).
