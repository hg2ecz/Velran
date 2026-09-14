<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# `f32` tömbök

A Velran numerikus számításokhoz korlátozott a bounded `f32` array tömböt biztosít:

```vrn
let mut samples = vec![0.0f32; 4096];
samples[0] = 1.0f32;
let first = samples[0];
let n = samples.len();
```

Az a bounded `f32` array futásidejű numerikus konténer, adatbázis- vagy modellmezőként nem használható. A foglalás beleszámít a request memória-budgetbe. Egy tömb jelenlegi biztonsági maximuma 1 048 576 elem. Az indexelés bounds-checkelt; negatív vagy túl nagy index fail-closed hibát ad.

A módosíthatóság Rust-szerűen explicit: a módosított tömb `let mut` deklarációt kap, az elemek írása pedig közvetlen assignment. A compiler továbbra is ellenőrzi a típust, a bounds feltételeket és az erőforrás-elszámolást a natív lowering előtt.
