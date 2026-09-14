<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Pure függvény String-lista példa

Immutable Rust-szerű `&[String]` pure-függvény paramétert és owned `Vec<String>` visszatérést mutat.
A generált natív reprezentáció megosztott, immutable ownershipot használ; nincs raw-pointer ABI.
