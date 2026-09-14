<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Korlátos request-kollekciók

A Velran a külső kollekció elemszámát biztonsági határként kezeli. A `Vec<String>` request mezők alapból korlátosak, és a korlát explicit szűkíthető.

```vrn
#[page]
fn search(ctx: PageContext, tags: Vec<String>) -> Result<Json, PageError> {
    return Ok(json(tags.len()));
}

route search GET "/search"
    query tags<Vec<String>>
    validate tags items 1 16
    public => search;
```

Ha nincs explicit `items` szabály, a biztonságos alapértelmezés `items 1 64`. Az explicit szabály ezt helyettesíti, nem egy második limitként rakódik rá.

## Bemeneti forma

- Query és form esetén ismételt mezők: `?tags=rust&tags=web`.
- JSON esetén valódi string tömb: `{ "tags": ["rust", "web"] }`.
- A normál request-kollekció nem CSV-string, ezért nem kell kézi `split()` boilerplate.

A JSON tömb már a deszerializáláskor hard cap-et kap, mielőtt a handler elindulna. A route ezután ennél szűkebb üzleti limitet írhat elő.

## Biztonsági tulajdonságok

- Duplikált skalár mező továbbra is fail-closed hiba.
- Ismételt mező csak `Vec<String>` esetén fogadható el.
- A request-lista csak bizonyított cardinality bound mellett válik `Validated<Vec<String>>` értékké.
- A platform abszolút plafonja 256 elem, az alap route-limit 64.
- Az üres JSON string tömb a jelenlegi wire formatban tiltott; ha a mező jelen van, legalább egy elemet kell tartalmaznia.

A Velran nem ad korlátlan, request-adatból hajtott collection iterációt. Budgetelt `while` létezik verifikált compute kódhoz, de request-derived kollekció transform/iteráció csak úgy bővíthető, hogy a bizonyított elemszám-korlát megmaradjon vagy szűküljön.
