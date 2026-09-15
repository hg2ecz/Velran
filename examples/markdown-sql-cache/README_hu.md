<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# SQL-alapú Markdown renderelés opcionális HTML response cache-sel

Ez a példa az ajánlott Velran mintát mutatja adatbázisban tárolt Markdownhoz:

1. az adatbázisban az eredeti Markdown forrás maradjon;
2. a rekordot typed `#[query]` töltse be kötött `:id` paraméterrel;
3. a `String` a forrásszintű `commonmark::render(...)` modullal legyen renderelve;
4. a kész publikus HTML response opcionálisan a normál route cache-be kerülhet.

Az alkalmazás az [`main.vrn`](main.vrn) fájlban van. SQLite, PostgreSQL és MariaDB inicializáló SQL is található a könyvtárban.

## Útvonalak

A `GET /documents/:id<i64>` nincs cache-elve. Minden kérés az aktuális SQL rekordot olvassa és abból rendereli a Markdownt.

A `GET /documents-cached/:id<i64>` ezt használja:

```velran
cache public ttl 120
```

A route paraméter része a cache identitynek, ezért a különböző dokumentumazonosítók nem ugyanazt a HTML választ használják. Ha minden kérésnek azonnal az aktuális adatbázisértéket kell látnia, hagyd el a cache clause-t.

## Biztonsági tulajdonságok

Az SQL named bindot (`:id`) használ, nem stringből épített SQL-t. A Markdownt a Velranban megírt `commonmark.vrn` forrásmodul rendereli; az engine-ben nincs Markdown-specifikus direktíva. A modul csak az engine általános, típusos `SafeHtml` építőprimitívjein keresztül készíthet markupot. A Markdownban szereplő raw HTML escape-elt szövegként jelenik meg, a veszélyes link sémák pedig nem válnak trusted markuppá.

A public cache csak akkor való ide, ha a generált oldal publikus, és nem függ felhasználói/session/request titoktól. Az Velran compiler cache-safety ellenőrzést végez a publikus cache-elt route-okon.

## Compiler/nyelvi határ

A CommonMark forrásmodul normál Velran kódként fordul. A verified pure helperek, scalar paraméterek, branching, rekurzió és string borrow az általános nyelvi contract része: [`docs/hu/58-verifikalt-pure-fuggvenyek.md`](../../docs/hu/58-verifikalt-pure-fuggvenyek.md). Nincs Markdown-specifikus compiler útvonal.

## Cache frissesség

A route cache a renderelt publikus választ tárolja, nem egy második HTML példányt az adatbázisban. Így a Markdown marad a source of truth, és a HTML előállítása végig a framework security policy alatt marad.

Ha a dokumentumot maga az alkalmazás is módosítja, az író route-nak invalidálnia kell a cache-elt olvasó route-ot. Ha egy külső folyamat közvetlenül módosítja az SQL táblát, a cache-elt HTML a TTL lejártáig látható maradhat; ilyenkor ennek megfelelő TTL-t válassz, vagy ne használj route cache-t.

## Adatbázis inicializálás

SQLite esetén:

```sh
sqlite3 app.db < examples/markdown-sql-cache/sqlite.sql
```

A könyvtárban `postgresql.sql` és `mariadb.sql` is található.

## Forrásszintű modul

A `main.vrn` a mellette lévő fájlt `mod commonmark;` deklarációval emeli be. A példa átvételekor a `commonmark.vrn` fájlt is másold az alkalmazás mellé.
