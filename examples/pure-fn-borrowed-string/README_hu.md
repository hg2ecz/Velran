<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Borrowolt string pure függvény

A példa a Rust-szerű `&str` paramétert mutatja verified pure függvényben. A generated shard a framework bounded string reprezentációját használja; az alkalmazáskód nem kap raw pointer, filesystem, network, process, environment, thread, FFI vagy unsafe jogosultságot.

A borrowed-string helper a jelenlegi verified pure surface része: a támogatott scalar/owned/string-list/struct/`Option<T>`/`Result<T,E>` értékeket adhatja vissza, typed expression argumentummal hívhat scalar/borrowolt pure helpert, használhat Rust-szerű branchinget, és a generált pure-call depth/resource limiteken belül rekurzív is lehet. Lásd: [`docs/hu/58-verifikalt-pure-fuggvenyek.md`](../../docs/hu/58-verifikalt-pure-fuggvenyek.md).
