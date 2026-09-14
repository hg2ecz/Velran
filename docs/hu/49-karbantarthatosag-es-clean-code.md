<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Karbantarthatóság és clean-code modulhatárok

A Velran a compiler, runtime és server felelősségeit explicit modulhatárok mögött tartja. A repository strukturális guardja szándékosan szigorúbb a Rust fordítónál: attól, hogy a kód érvényes Rust, még megbukhat a clean-structure ellenőrzésen, ha egy façade fájl ismét egymástól független felelősségeket kezd gyűjteni.

## Típusos konfigurációs hibák

A runtime resource-profile létrehozása `String` helyett `ResourceProfileError` típussal tér vissza. A library-hívó így konkrét hibavariánsra tud illeszteni, míg a CLI/server határon továbbra is a `Display` szerinti emberileg olvasható szöveg jelenhet meg. Új konfigurációs library API-knál ugyanezt a mintát kell követni.

## Builtin metadata

A `BuiltinFunction` egy helyen tartja minden nyelvi builtin stabil metaadatait:

- forráskódbeli név;
- minimális és maximális argumentumszám;
- instruction-költség;
- request-state függőség;
- végrehajtási osztály (`Simple` vagy `Regex`).

A compiler ugyanezt használja általános aritásellenőrzésre, a runtime pedig budget-terhelésre és execution dispatchre. Új builtin esetén nem szabad külön párhuzamos név/költség/regex listát létrehozni más modulban.

## Tesztmodul-határok

A runtime crate root kis façade marad. A crate-private helpereket igénylő tesztek a `test_support` modulon keresztül importálnak, nem `lib.rs`-be tett teszt-specifikus wildcard importokon át. A kisebb határmodulok explicit importokat használnak `use super::*` helyett, így review során látható a tényleges függőség.

## Strukturális guard

Futtasd:

```sh
./tools/check-clean-structure.sh
./tools/check-positive-examples.sh
```

Megbízható Rust fejlesztői környezetben ezen felül:

```sh
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

A refaktor nem változtathatja meg a Velran külső szemantikáját. Ha egy modul kinövi a guardot, koherens felelősséget kell kiszervezni, nem indoklás nélkül megemelni a limitet.
