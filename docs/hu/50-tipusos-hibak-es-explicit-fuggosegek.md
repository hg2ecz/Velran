<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Típusos hibák és explicit modulfüggőségek

Az R8 karbantarthatósági kör új nyelvi funkció nélkül folytatja a clean-code refaktort.

## Típusos erőforráslimit-hibák

A server `resource_limits` modul `apply` API-ja többé nem `Result<_, String>` típust ad vissza. A hibákat a `ResourceLimitError` külön kategóriákba rendezi: hibás operátori konfiguráció, nem támogatott platformfunkció, operációs rendszer/I/O hiba és CPU-kvóta túlcsordulás.

A konzolon látható hibaüzenetek megmaradnak, de a hiba most programból is osztályozható, és az eredeti I/O hiba elérhető a `std::error::Error` source láncon keresztül.

## Külön rate-limit modul

A route rate limiter kikerült a szerver főfájljából a `server/src/rate_limit.rs` modulba. A `RateLimitError` külön kezeli az ismeretlen policyt, a hiányzó hitelesített felhasználót, az órahibát, a backendhibát, a lock poison állapotot és a memóriabeli limiter kapacitáshibáját.

A külső HTTP viselkedés változatlanul fail-closed: belső limiterhiba esetén a kliens továbbra is service-unavailable választ kap belső részletek nélkül.

## Explicit compiler-függőségek

A kisebb collection parser modulok csak azokat a típusokat és helper API-kat importálják, amelyeket ténylegesen használnak. Nem támaszkodnak többé `use super::*` implicit függőségekre.

A strukturális guard ezt külön ellenőrzi, így későbbi crate-root refaktorok kevésbé tudnak rejtetten más modulokat eltörni.
