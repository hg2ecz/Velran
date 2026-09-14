<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Modulimportok

A 6. iteráció determinisztikus Velran modulfelületének kanonikus példája.

- A `mod child;` a deklaráló modulhoz képest relatív.
- A `pub mod child;` sibling namespace felé is megnyit egy almodult.
- A `crate::`, `self::` és `super::` explicit útvonalhorgony.
- A `use path as alias;` namespace-prefixet rövidít, teljes saját Rust scope/type checker nélkül.
- A `pub use` egyelőre szándékosan tiltott; a publikus API a definiáló modulban marad explicit.
