<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Helyi package-ek
Security-first helyi path library példa. Az `velran.toml` csak explicit relatív path dependencyt enged. A library stabil package namespace-et kap, alapból private, és csak ordinary pure/API kódot exportálhat; a web/server authority az alkalmazásban marad. Távoli registry/git/build script és tranzitív dependency egyelőre szándékosan nincs.
