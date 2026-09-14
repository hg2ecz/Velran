<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Típusos JSON válasz

A példa a `Json<T>` kéréskötést és a `Result<Json<T>, PageError>` választ mutatja.
A generált alkalmazási shard csak korlátozott, típusos bináris válaszkeretet állít elő; a JSON szerializálás és escaping a keretrendszer feladata marad.
