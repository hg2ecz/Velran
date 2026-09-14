<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Budgetelt `while` ciklus és explicit lokális értékadás

A Velran támogat erőforrás-számlált compute ciklust numerikus feladatokhoz. A `while` feltételének `bool` típusúnak kell lennie; minden feltételvizsgálat és törzsutasítás fogyasztja a request instruction budgetet. A budget elfogyása a szokásos instruction-limit hibával állítja meg a futást, tehát nem maradhat korlátlanul futó worker ciklus.

```vrn
let mut i = 0;
let mut samples = vec![0.0f32; 4096];

while i < samples.len() {
    samples[i] = 1.0f32;
    i = i + 1;
}
```

A módosítható lokális `let mut` deklarációt kap, és normál Rust-szerű assignmenttel frissíthető. A statikus típus nem változhat: `i64` lokálisba `f32` kifejezés nem írható. A tömbelem módosítása `array[index] = value`, futásidejű bounds checkkel.

Az összehasonlító operátorok: `==`, `!=`, `<`, `<=`, `>`, `>=`. Rendezési összehasonlítás jelenleg `i64`, `f32` és `Decimal` típusokra használható; egyenlőség `String` és `bool` esetén is. Vegyes numerikus típusok implicit összehasonlítása szándékosan tiltott, például `1 < 2.0f32` fordítási hiba.

A `while` törzsében létrehozott `let` lokálisok compiler szempontból a compute blokkhoz tartoznak. Külső lokális csak akkor módosítható, ha `let mut` deklarációval jött létre. Egymásba ágyazott `while` ciklusok is használhatók, ugyanazt az instruction budgetet fogyasztva.

A funkció determinisztikus, korlátozott alkalmazásoldali számításra készült, például numerikus transzformációkhoz. Nem kerüli meg a request timeoutot, az allocation limiteket vagy a named resource profile-okat.
