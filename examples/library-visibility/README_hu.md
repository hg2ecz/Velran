<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Könyvtári láthatóság

A példa a nagyobb projektekhez bevezetett Rust-szerű Velran API-határt mutatja:

- a library-elemek alapból privátak;
- a `pub struct`, `pub fn`, publikus mezők és `pub` inherent metódusok alkotják az explicit modulok közötti API-t;
- maga az `impl` blokk nem lehet `pub`;
- a webes callable-ok authorityját továbbra is route/capability policy adja, nem a Rust visibility.

A `Summary::hidden` mező és a `Summary::hidden_value()` metódus szándékosan privát implementációs részlet.
