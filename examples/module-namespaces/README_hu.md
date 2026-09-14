<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Modulnévterek

Ez a példa az Velran V1 modulnévtér-modelljét mutatja be.

A `main.vrn` explicit módon deklarálja a teljes forrásgráfot:

```vrn
mod catalog;
mod catalog::queries;
mod catalog::pages;
```

A feloldás application-root relatív és kanonikus:

```text
catalog.vrn          -> catalog
catalog/queries.vrn  -> catalog::queries
catalog/pages.vrn    -> catalog::pages
```

Egy modul betöltése nem emeli a deklarációit globális névtérbe. Modulhatáron át ezért kvalifikált név kell: `catalog::Product`, `catalog::queries::recent(...)` és `catalog::pages::index`.

Egy modulon belül a saját deklarációk rövid lokális neve továbbra is használható. A route-ok alkalmazásszintű HTTP-azonosítók, és külön fogalmat alkotnak a kód névterétől.

Ellenőrzés:

```bash
cargo run --locked -q -p velran-cli -- check examples/module-namespaces/main.vrn
```
