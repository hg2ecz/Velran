<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Velran V1 kiadási jegyzet

**A projekt hivatalos teljes neve:** Velran — a security-first web application language and server with Rust syntax.

Az Velran V1 ennek a release tree-nek a Rust-first, native-only webalkalmazás-nyelv/runtime/server alapvonala. A canonical dokumentáció angol; a magyar fejlesztői és operátori anyag a `docs/hu/` alatt található.

## Nyelv és végrehajtás

- A hétköznapi compute kód Rust-szerű surface-et használ: `fn`, `let`, `let mut`, közvetlen/compound assignment, Rust primitív- és konténertípusok, metódusszintaxis és `vec![...]`.
- Velran-specifikus szintaxis ott marad, ahol webes vagy security szemantikát hordoz: route, typed request schema, authorization policy, HTML/template boundary, resource contract és named capability.
- Az alkalmazás-végrehajtás: verifikált IR -> generált safe Rust -> `rustc` -> immutable `cdylib` -> atomikus aktiválás. Nincs VM/interpreter fallback. Nem támogatott natív lowering vagy host ABI esetén a candidate fail-closed módon elutasításra kerül, az előző valid generáció aktív marad.

## Security és üzemeltetés

- Explicit route access policy (`public`, auth/permission/MFA/critical policyk).
- Típusos és korlátos request input, domain/nominális érték, resource budget, security-event és redaction alapok.
- Tranzakciós source reload, content-addressed natív artifact, supply-chain capability/provenance ellenőrzés és reprodukálható `--locked` dependency feloldás.
- Production konfiguráció, migráció, backup/restore/rollback és release evidence a támogatott üzemeltetési modell része.

## Verifikáció

A release tree elvárt ellenőrzése:

```bash
cargo build --release --locked
./verify.sh
```

Aktuális security állapot: [`SECURITY-STATUS.md`](SECURITY-STATUS.md). Natív lefedettség: [`tests/NATIVE_STATUS.md`](tests/NATIVE_STATUS.md).

## 6. iteráció: determinisztikus modul/import felület

- A child `mod` deklarációk most a deklaráló modulhoz képest relatívak; a `crate::`, `self::` és `super::` explicit horgony.
- A `pub mod` valódi nested library visibility-határ, de nem változtatja meg a route/capability authorityt.
- A `use path as alias;` explicit namespace-prefix alias; a wildcard/bare-symbol import és a `pub use` továbbra is fail-closed.
- A modul/import parsing központi helye a `compiler::module_header`; a source loader felel a confined fájlrendszer-bejárásért.
- Bekerült a release-gated `examples/module-imports/` kanonikus példa és a statikus regressziós guard.
- Egy test-only `Result<_, String>` native-build API typed error lett, két túlnőtt executable-IR/codegen felelősséget pedig szétválasztottunk a budget mesterséges emelése helyett.

## Visszaszámlált 5B → 1 iterációk

A lezáró visszaszámlálás konszolidálta a Rust-szerű package/objektum felületet, szűk biztonságos re-export szemantikát adott, gyorsította a verifikált natív hot-pathokat, csökkentette a compiler/native pipeline overheadet, majd release-hardening és clean-code szétválasztással zárult anélkül, hogy a webes authority felület bővült volna. A jelenlegi fontos szerződések közé tartozik a `Type::new(...)` jellegű associated function, a támogatott inherent-impl részhalmazon belüli `Self`, a cross-package publikus struct/metódus API, a kizárólag package-rootból engedett explicit modul-re-export, a chunkolt HTML escaping, a borrowed read-only string/map fast-path és a security-kritikus határokon csökkentett panic-használat.

### Ellenőrzött fejlesztési mérföldkő

Ez a forrásfa valódi Rust toolchainnel elérte az **Ellenőrzött fejlesztési mérföldkövet**: a jelenlegi fán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. A repository-szintű verifikációs állapot ezzel ezen a mérföldkövön véglegesíthető.

A production release elfogadása ettől külön döntés. A célkörnyezetben továbbra is teljesíteni kell a [`RELEASE-CHECKLIST.md`](RELEASE-CHECKLIST.md) deployment-, recovery-, secret/TLS/reverse-proxy, operátori, valamint szükséges környezetfüggő performance/security evidence pontjait.
