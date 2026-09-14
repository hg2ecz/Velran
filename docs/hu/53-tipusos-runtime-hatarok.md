<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Típusos runtime hibahatárok a szerverben

A szerver a leaf határokon típusos hibákat tart meg, és csak a legfelső startup aggregációs határon törli a konkrét típust.

## Publikus oldal cache

A `PublicPageCache` már `PublicCacheError` típust ad `String` helyett. Külön hibakategória marad a rendszerórára, a poisoned in-process lockokra, a Redis/data backendre, a hibás UTF-8 vagy numerikus generation értékre, valamint a JSON szerializációra.

## Órahatár

A `unix_secs()` `ClockError` típust ad. A Unix epoch előtti rendszeróra nem egyszerű szöveges hibává alakul.

## Upload érték összeállítása

A `build_upload_runtime_value()` `UploadRuntimeError` típust használ, különválasztva a storage olvasási hibát, a képvalidációt és a validált image reference létrehozási hibáját.

## Listener és signal határok

A HTTP redirect listener és a shutdown signal helper közvetlenül `std::io::Error` típust ad. Itt nincs szükség külön enumra, mert a leaf műveletek eleve egyetlen pontos standard könyvtári hibatípust használnak.

## Aggregációs szabály

A `startup::run()` továbbra is szándékos alkalmazáshatár, ahol heterogén hibák `Box<dyn Error>` alatt találkozhatnak. Leaf modul ne vezessen be boxed vagy stringly hibát pusztán kényelmi okból.
