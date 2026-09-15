<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Pure függvény owned String visszatéréssel

Verified pure `fn` példa `&str` bemenettel és owned `String` visszatéréssel, raw pointer és ambient authority nélkül.

Az aktuális pure-call, paraméter-, rekurzió- és resource-safety contract: [`docs/hu/58-verifikalt-pure-fuggvenyek.md`](../../docs/hu/58-verifikalt-pure-fuggvenyek.md).
