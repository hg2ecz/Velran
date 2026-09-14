<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# String műveletek

Nyisd meg a `/strings?text=%20Velran%20rust%20lang%20` URL-t a típusos string builtinok kipróbálásához.

A példa Unicode-tudatos karakterhosszt, trimet, kis-/nagybetűsítést, részszöveg-vizsgálatokat és erőforrás-budgetelt cserét mutat. A String eredményt előállító builtinok memóriafoglalása beleszámít a runtime allocation limitbe.
