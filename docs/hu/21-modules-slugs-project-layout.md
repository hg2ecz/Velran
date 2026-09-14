<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Projektstruktúra, modulok és névterek

A Velran modulok application-root relatív névterek. A `mod` betölt egy source unitot, de annak deklarációit nem emeli be sem az aktuális, sem a globális névtérbe.

## Kanonikus leképezés

```text
main.vrn                alkalmazás-root
models.vrn              models
pages/article.vrn       pages::article
admin/users/edit.vrn    admin::users::edit
```

Pontosan egy forrásleképezés van: `a::b` → `<app-root>/a/b.vrn`. Nincs `mod.vrn` alternatíva.

```velran
mod models;
mod pages::article;
```

A modulútvonal mindig application-root relatív. A `../`, `./`, `self::`, `super::` és `crate::` nem támogatott.

A `mod` az alkalmazás source graph tagságát deklarálja, nem lexikális importlista. Ha a `main.vrn` vagy egy másik már betöltött source unit betöltött egy modult, bármely betöltött source unit hivatkozhat rá az abszolút namespace útvonalával. A top-level modulgráfot célszerű a `main.vrn`-ben összeállítani.

A `mod` top-level source-graph deklaráció, ezért az adott source unit normál deklarációi előtt kell szerepelnie. A könyvtárakat a compiler nem járja be automatikusan: a `mod catalog;` csak a `catalog.vrn` fájlt tölti be, a `catalog/queries.vrn` fájlhoz külön `mod catalog::queries;` szükséges. V1-ben nincs olyan `use` vagy wildcard import sem, amely egy másik modul neveit az aktuális scope-ba emelné.

## Szimbólumazonosság

A nem root modulban deklarált enum, model, form, query, component, layout, page és action a modul névterébe kerül.

`pages/article.vrn`:

```velran
#[page]
fn show(ctx: PageContext, slug: Slug) -> Result<Html, PageError> {
    return Ok(html {<h1>{{ slug }}</h1>});
}
```

A teljes neve:

```text
pages::article::show
```

A saját modulon belül a lokális deklaráció rövid névvel is hivatkozható, és az aktuális modul névterében oldódik fel. Modulhatár átlépésekor kötelező a teljes namespace. Külső hivatkozás:

```velran
route articleShow GET "/articles/:slug<Slug>" public => pages::article::show;
```

A `mod pages::article;` tehát nem hoz létre globális `show` nevet.

A route neve továbbra is alkalmazásszintű HTTP-azonosító, ezért globálisan egyedi marad és nem kap automatikusan modulprefixet.

## Modulok közötti használat

```velran
// queries.vrn
#[query]
fn byId(db: Db, id: i64) -> Result<models::Article, DbError> sql {
    SELECT id, title FROM articles WHERE id = :id
}
```

```velran
// pages.vrn
#[page]
fn show(ctx: PageContext, db: Db, id: i64) -> Result<Html, PageError> {
    let article = queries::byId(db, id)?;
    return Ok(html {<h1>{{ article.title }}</h1>});
}

route articleShow GET "/articles/:id<i64>" public => pages::show;
```

A teljes név szándékosan explicit: a dependency látható marad, és két modul azonos helyi neve nem ütközik.

## Domain objectek

A modulnévtér és a domain-object member jelölés együtt használható:

```velran
content::article::Article.bySlug(...)
```

A modul separator `::`, a domain-object member separator továbbra is `.`.
