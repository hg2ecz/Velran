<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# `f32` numerikus értékek

A Velran Rust-szerű `f32` típusa IEEE-754 egyszeres pontosságú lebegőpontos számításokra szolgál. Nem azonos a `Decimal` típussal: a `Decimal` pontos decimális/üzleti értékekhez való, az `f32` pedig numerikus algoritmusokhoz.

Az `f32` literál explicit `f32` suffixet igényel:

```vrn
let x = 1.25f32;
let y = -0.5f32;
let z = x * y + 2.0f32;
```

A suffix nélküli törtszám fordítási hiba. Implicit kevert aritmetika sincs: az `1 + 0.5f32` hibás, így a pontosságváltás nem történhet észrevétlenül.

A Velran csak véges `f32` értéket enged. `NaN` és pozitív/negatív végtelen inputként elutasításra kerül, a nem véges eredményt előállító aritmetika pedig fail-closed runtime hibát ad. A nullával osztás szintén runtime hiba.

Az `f32` külön verifikált numerikus érték marad, nem `Decimal` konverzión keresztül fut. A generált safe Rust a támogatott `+`, `-`, `*`, `/`, `%` műveleteket közvetlenül egyszeres pontossággal végzi, a nem véges eredményeket fail-closed módon elutasítva.

Typed HTTP/form/query inputban is használható `f32`. A DB kompatibilitási réteg jelenleg kanonikus szövegként tárolja, hogy a támogatott backendek között azonos parse-szemantika maradjon.

A korlátos tömb, indexelés, budgetelt ciklus, matematikai metódusok és monoton időmérés már része a verifikált natív részhalmaznak; lásd a következő numerikus fejezeteket és az FFT4096 példát.
