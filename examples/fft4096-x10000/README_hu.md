<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# FFT4096 x10000 valódi függvényhívás benchmark

A benchmark ugyanazt a 4096 pontos radix-2 FFT-t 10 000-szer futtatja egy valódi,
felhasználó által definiált pure Velran/Rust-szerű függvényen keresztül:

```rust
#[inline(never)]
fn fft4096(real: &mut [f32; 4096], imag: &mut [f32; 4096]) { ... }
```

Az egyedi időmérés kizárólag az `fft4096(&mut real, &mut imag)` hívást méri. A buffer
visszaállítás, a futásonkénti minimális perturbáció, a checksum és a HTML renderelés kívül marad.

Security first korlátok ebben az első pure-fn iterációban:

- csak `&mut [f32; N]` paraméter engedett;
- csak verified lokális compute statementek használhatók;
- nincs DB/network/filesystem/env/process/thread/FFI/unsafe authority;
- ugyanaz a tömb nem adható át kétszer mutable paraméterként;
- a fix tömbméretet a compiler/IR ellenőrzi;
- a helper fuel/allocation fogyasztása a hívó route budgetjét terheli;
- csak `#[inline]`, `#[inline(always)]`, `#[inline(never)]` attribútum engedett.
