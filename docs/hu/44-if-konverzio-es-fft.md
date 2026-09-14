<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Compute `if`, explicit `f32` konverzió és FFT4096

A Velran compute kód budgetelt feltételes blokkot támogat, implicit `else` ág nélkül:

```vrn
let mut x = 3;
if x < 10 {
    x = x + 1;
}
```

A feltétel típusa kötelezően `bool`. A blokk utasításai ugyanazt az instruction- és allocation-budgetet fogyasztják, mint a `while` compute kód. A blokkon belül deklarált lokális változók compile-time szinten blokklokálisak; meglévő, `let mut` lokális normál Rust-szerű assignmenttel módosítható.

## Explicit i64 -> f32 konverzió

A ``i64 as f32`` az egész index/control értékek és a bináris lebegőpontos számítás közötti explicit átjáró:

```vrn
let i = 64;
let phase = i as f32 * 0.125f32;
```

Nincs implicit `i64`/`f32` konverzió. A az `as f32` nagy egész számoknál szándékosan explicit, potenciálisan pontosságvesztő IEEE-754 konverzió; ahol pontos egész szemantika kell, ott `i64` maradjon.

## FFT4096 benchmark

Az `examples/fft4096/main.vrn` teljes iteratív radix-2 Cooley-Tukey FFT-t valósít meg Velranban, két korlátos `f32` tömbbel a valós és képzetes komponensekhez. Nem hív natív FFT könyvtárat.

A determinisztikus bemeneti jel egy egységnyi amplitúdójú 64-es és egy fél amplitúdójú 256-os binű szinuszt tartalmaz. Normalizálatlan transzformációnál a várt magnitúdók megközelítőleg 2048 és 1024. A példa kiírja:

- az FFT `std::time::Instant::now()` segítségével mért idejét;
- a 64-es bin magnitúdóját;
- a 256-os bin magnitúdóját;
- egy széles correctness tartomány eredményét.

Az időmérés a jelgenerálás után indul, és a bit-reversal valamint az összes FFT stage idejét tartalmazza. Compute-heavy példához megfelelően magas instruction budget szükséges. Teljesítménymérésnél külön mérjük az első natív shard fordítás/betöltés idejét és a későbbi cache-hit kéréseket.
