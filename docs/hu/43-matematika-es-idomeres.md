<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Numerikus operátorok, `f32` matematika és monoton időmérés

A Velran ellenőrzött numerikus operátorokat és budgetelt matematikai builtin készletet ad az alkalmazás- és compute kódhoz.

## Operátorok

```vrn
let osszeg = 10 + 3;
let kulonbseg = 10 - 3;
let szorzat = 10 * 3;
let hanyados = 10 / 3;
let maradek = 10 % 3;

let balra = 3 << 2;
let jobbra = 24 >> 2;
let maszkolt = 12 & 10;
let valtott = 12 ^ 10;
let egyesitett = 12 | 3;

let engedelyezett = kesz && jogosult;
let hasznalhato = cachelt || friss;
let tiltott = !engedelyezett;
```

A `+`, `-`, `*`, `/` és `%` azonos típusú `i64`, `f32` és `Decimal` operandusokra használható. A `String + String` továbbra is konkatenáció. A shift és bitenkénti operátorok (`<<`, `>>`, `&`, `^`, `|`) csak `i64` értékekre érvényesek. A `&&`, `||` és `!` `bool` típust vár; a `&&` és `||` short-circuit módon működik, tehát szükség esetén a jobb oldal ki sem értékelődik.

A precedencia szorosabbtól lazább felé: unáris `!`; `* / %`; `+ -`; shiftek; összehasonlítások; bitenkénti `&`, `^`, `|`; logikai `&&`; logikai `||`. Ha az olvashatóság javul, használj zárójelet.

Az aritmetika ellenőrzött marad. Integer overflow, nullával osztás/maradékképzés, nem véges `f32` eredmény és hibás shift-szám fail-closed hibát ad, nem csendes wraparoundot.

## `f32` matematikai metódusok

```vrn
let x = 0.5f32;
let s = x.sin();
let c = x.cos();
let gyok = 4.0f32.sqrt();
let abszolut = (-3.5f32).abs();
let termeszetes = 2.0f32.ln();
let tiz_alapu = 1000.0f32.log10();
let kettes_alapu = 8.0f32.log(2.0f32);
let exponencialis = 1.0f32.exp();
let hatvany = 2.0f32.powf(8.0f32);
let kerek = 2.6f32.round();
let le = 2.6f32.floor();
let fel = 2.1f32.ceil();
```

Szignatúrák:

```text
f32.sin() -> f32
f32.cos() -> f32
f32.sqrt() -> f32
abs(i64) -> i64
f32.abs() -> f32
f32.ln() -> f32
f32.log10() -> f32
f32.log(f32) -> f32
f32.exp() -> f32
f32.powf(f32) -> f32
f32.round() -> f32
f32.floor() -> f32
f32.ceil() -> f32
i64 as f32
```

Minden `f32` eredménynek végesnek kell maradnia. A hibás tartományú műveletek, például `(-1.0f32).sqrt()` vagy `(-1.0f32).ln()`, hibával leállnak NaN vagy végtelen érték létrehozása helyett.

## Monoton időmérés

```vrn
let started = std::time::Instant::now();
// mért munka
let elapsed = started.elapsed().as_nanos();
```

A `std::time::Instant::now()` processzen belüli monoton eredethez képest ad vissza nanoszekundumot `i64` típusként. Eltelt idő mérésére való, nem dátum/időbélyeg előállítására. A compiler nem enged public cache-t olyan oldalnál, amelynek kimenete ettől az órától függ.

A matematikai builtinok súlyozott instruction-budget költséget kapnak. A transzcendens műveletek drágábbak az egyszerű aritmetikánál, így a számításigényes kód is a konfigurált erőforráskorlátok alatt marad.

Futtatható példa: `examples/numeric-operators/main.vrn`.

## Kapcsolódó nyelvi szabályok

A fenti `let`, assignment és `return` formák egyszerű statementek, ezért `;` zárja őket; egy kifejezésen belüli sortörés nem terminátor. Lásd: [Statement terminátorok](55-statement-terminatorok.md). A `String + String` konkatenáció mellett a teljes Unicode-tudatos String API: [String műveletek](45-string-muveletek.md).
