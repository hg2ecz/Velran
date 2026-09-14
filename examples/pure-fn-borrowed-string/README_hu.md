<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Borrowolt string pure függvény

A példa a Rust-szerű `&str` paramétert mutatja verified pure függvényben. A generated shard a framework bounded string reprezentációját használja; az alkalmazáskód nem kap raw pointer, filesystem, network, process, environment, thread, FFI vagy unsafe jogosultságot.

Ebben az iterációban a helper csak scalar értéket ad vissza. Az owned `String`, collection, struct, `Option<T>` és `Result<T,E>` return szándékosan csak a következő, ownership-biztos lépésben kerül be.
