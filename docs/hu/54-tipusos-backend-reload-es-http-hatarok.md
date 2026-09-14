<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Típusos backend, reload és HTTP hibahatárok

Az R12 a clean-code hibahatár-tisztítást folytatja funkcióváltozás nélkül.

## Backend/runtime felépítés

A `backend_support.rs` listener-bind, domain-runtime és hosting-runtime API-jai most `BackendSupportError` típust adnak. A compiler-, resource-profile-, static-prefix-, storage- és I/O-hibák így nem vesznek el egy `Box<dyn Error>` mögött.

A konfigurációs ütközések — például foglalt media/health route, hiányzó upload storage vagy elégtelen AppFs jogosultság — külön hibavariánsok.

## Source reload

A `source_reload.rs` candidate-validációja, felépítése, commitja és cache invalidálása `SourceReloadError` típust használ. Külön kezelhető a backend építési hiba, rate-policy validáció, cache invalidálás, poisoned hosting lock, túl nagy cache TTL, illetve a hiányzó cache/database/auth capability.

## A dispatch valójában infallible

A `dispatch_upload` és `dispatch_buffered` korábban `Result<DispatchOutcome, Box<dyn Error>>` típust adott, miközben minden request-szintű hibát már HTTP válasszá alakított, és nem volt propagált error ág. Most közvetlenül `DispatchOutcome`-ot adnak.

Clean-code szabály: ne jelöljünk fallible API-t, ha a hiba már a domain eredmény részeként van reprezentálva.

## Response írás

A `write_response_with_timeout` kizárólag aszinkron I/O-t és timeoutot kezel, ezért közvetlenül `std::io::Result<()>` a visszatérési típusa. A timeout `io::ErrorKind::TimedOut`.

## Határszabály

A leaf modulok a lehető legszűkebb értelmes hibatípust adják. `Box<dyn Error>` csak tudatos alkalmazás-szintű aggregációs határon indokolt, például a `startup::run` esetén.
