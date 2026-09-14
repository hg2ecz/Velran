<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Validált input adatfolyam

A Velran külön kezeli a belső/megbízható, a külső, illetve a route-szerződésen már ellenőrzött skalár értékeket.

```text
Trusted      belső/compiler/runtime tulajdonú adat
Validated    ellenőrzött route-határon átjutott külső adat
Untrusted    még nem validált külső adat
```

A fejlesztőnek normál handlerben nem kell `Untrusted<T>` vagy `Validated<T>` wrapper-eket kiírnia. A route maga a trust boundary.

A `String` query/form/json és most már path inputok is biztonságos alapértelmezett felső hosszkorlátot kapnak. Szűkebb domain-szabály egyszerűen megadható:

```velran
route create POST "/notes"
    form title<String>
    validate title length 1 160
    public => create;
```

Az olyan típusok, mint `Email`, `Url`, `Slug`, `i64`, `bool`, `Uuid`, `Date`, `DateTime` és `Decimal` a saját dekóderük/normalizálójuk sikeres lefutása után kapnak validációs proofot. Hibás érték nem jut el a handlerig.

A tranzakciós query-k `SEC-DATA-003` hibával elutasítják a még `Untrusted` skalár argumentumokat. Ez különösen fontos például upload metaadatnál: a kliens által megadott fájlnév nem írható közvetlenül adatbázisba.

Az authorization proof ettől független: a validáció azt bizonyítja, hogy az adat megfelel a szerződésnek; az authorization azt, hogy az adott principal a konkrét objektumot módosíthatja.

Alapelv:

> A validáció proof, amelyet a boundary állít elő, nem egy application-code konvenció, amelyre emlékezni kell.
