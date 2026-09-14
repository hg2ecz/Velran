<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Secret-fogyasztói szerződések

A Velran a `Secret<T>` típust fordítási idejű adatklasszifikációként kezeli, nem kozmetikai alias-ként.
Secret érték csak olyan sinkbe léphet át, amely explicit módon deklarálja, hogy `Secret<T>` értéket fogyaszt.

## Explicit query sink

Ha egy query szándékosan secretet használ, ezt a signature-ben ki kell mondani:

```vrn
#[query]
fn lookupByToken(
    db: Db,
    token: Secret<String>
) -> Result<Option<Credential>, DbError> sql {
    SELECT id, token FROM credentials WHERE token = :token
}
```

`Secret<String>` sima `String` paraméterbe adása fordítási hiba (`SEC-DATA-006`).
Sima `String` adat `Secret<String>` paraméterbe adása szintén hiba (`SEC-DATA-007`).
Az annotáció tehát kétirányú szerződés: secret nem szivároghat nem klasszifikált sinkbe, és közönséges adat sem tehet úgy, mintha secret capability lenne.

A nominális típusidentitás ettől függetlenül megmarad: `Secret<ApiToken>` esetén a pontos `ApiToken` domain típus és a secret klasszifikáció is szükséges.

## Web handler paraméter nem secret capability

Request paraméter page/action signature-ben nem deklarálható `Secret<T>` vagy `Sensitive<T>` típussal (`SEC-DATA-009`). A request adat trust boundary input. Érzékeny vagy secret érték explicit módon klasszifikált modellmezőből vagy más trusted capabilityből származhat.

## Audit boundary

A business audit `object_id`, `from` és `to` értékeinél a `Sensitive<T>` és `Secret<T>` tiltott (`SEC-DATA-008`). Auditba stabil azonosító, státusz/enum átmenet vagy szándékosan redaktált publikus adat kerüljön.

Így a hosszú élettartamú audit storage nem válik véletlen credential- vagy PII-sinkké.

## Nyelvtervezési szabály

A későbbi secret-fogyasztó API-kra ugyanaz az alapelv vonatkozik:

> Secretet csak olyan API fogyaszthat, amelynek típuskontraktja explicit kimondja, hogy secretet fogyaszt.

A log, debug, URL, response body, public projection és audit nem secret consumer. Outbound autentikáció és kriptográfiai primitívek keskeny, explicit secret-consuming capabilityket kapjanak általános String API helyett.
