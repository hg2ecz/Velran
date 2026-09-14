<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Nominális domain típusok

A Velran domain típusai valódi nominális típusok, nem egyszerű aliasok.

```vrn
type UserId = i64 { range 1 999999999; }
type ArticleId = i64 { range 1 999999999; }
```

Bár mindkettő runtime reprezentációja `i64`, fordításkor külön típusok. Ez hibás:

```vrn
#[query]
fn loadUser(db: Db, id: UserId) -> Result<Option<User>, DbError> sql {
    SELECT id FROM users WHERE id = :id
}

let user = loadUser(db, articleId)?;
```

A compiler `SEC-TYPE-001` hibát ad, és az üzenetben a forrásbeli `UserId` illetve `ArticleId` neveket mutatja. Nyers `i64` literál sem alakul át csendben `UserId` értékké.

A nominális azonosság megmarad route paramétereken, handler paramétereken, modellmezőkön, query paramétereken, lokális aliasokon, typed redirecteken és typed HTML route helperen keresztül. Ez megakadályozza a különböző objektumazonosítók véletlen összekeverését, és erősíti az authorization-proof modellt.

A biztonságos, reprezentációt fogyasztó műveletek továbbra is kényelmesek. Egy `Username = String { ... }` használható HTML escapingben, `len`-nel, `splitBounded`-dal és más biztonságos string műveletekkel. Ezek a `String` reprezentációt fogyasztják, de nem gyártanak automatikusan új validált `Username` értéket.

Ha egy String domain csak például `pattern` constraintet deklarál, a Velran automatikusan hozzáad egy fail-closed `length 0 4096` korlátot. Ha az üzleti domain ennél szűkebb, azt továbbra is érdemes explicit deklarálni.

Runtime oldalon a domain értékek az alap skalár reprezentációt használják. A nominális azonosság a lefordított program típuskontraktjában él, nem heap wrapperként, ezért az erősebb típusbiztonság nem ad értékenkénti allokációs költséget.
