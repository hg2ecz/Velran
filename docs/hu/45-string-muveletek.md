<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# String műveletek

A Velran típusos, Unicode-tudatos string-compute magot ad az alkalmazáslogikához. Ezek a műveletek budgetelt generált Rust kódra fordulnak, ezért az instruction- és memóriaelszámolást nem kerülik meg; nincs bytecode fallback.

## API

```vrn
let cleaned = text.trim();
let left = text.trim_start();
let right = text.trim_end();
let low = cleaned.to_lowercase();
let high = cleaned.to_uppercase();
let n = cleaned.chars().count();
let has = low.contains("rust");
let prefix = low.starts_with("vrn");
let suffix = low.ends_with("lang");
let rewritten = low.replace(" ", "-");
let part = substring(cleaned, 1, 3);
let first = indexOf(cleaned, "vrn");
let last = lastIndexOf(cleaned, "a");
let ch = charAt("Velran", 1);
let repeated = "vrn".repeat(3);
let pieces = splitBounded(cleaned, ",", 4096);
```

Szignatúrák:

```text
String::chars().count() -> i64
String::trim() -> String
String::trim_start() -> String
String::trim_end() -> String
String::to_lowercase() -> String
String::to_uppercase() -> String
String::contains(String) -> bool
String::starts_with(String) -> bool
String::ends_with(String) -> bool
String::replace(String, String) -> String
splitBounded(String, String, i64) -> Vec<String>
substring(String, i64) -> String
substring(String, i64, i64) -> String
indexOf(String, String) -> i64
lastIndexOf(String, String) -> i64
charAt(String, i64) -> String
String::repeat(i64) -> String
```

A `.chars().count()`, `substring`, `indexOf`, `lastIndexOf` és `charAt` Unicode skalárérték-pozíciókkal dolgozik, nem UTF-8 byte indexekkel. Az `indexOf` és `lastIndexOf` `-1` értéket ad, ha nincs találat. A `substring(text, start)` a `start` pozíciótól adja vissza a string végét; a háromparaméteres forma karakterhosszt kap, és a string végénél levágja a tartományt. Negatív vagy érvénytelen index fail-closed hibát ad.

A kis- és nagybetűsítés Unicode-tudatos. A `replace` nem enged üres keresőszöveget, a `split` pedig üres delimitert; egy split legfeljebb 4096 elemet eredményezhet. A `repeat` nem enged negatív ismétlésszámot, és az allokáció előtt bekerül a request memória-budget elszámolásába.

## Erőforrás-elszámolás

A String eredményt készítő builtinok az eredmény előállítása előtt lefoglalják a becsült/kiszámított kimeneti méretet a request és resource-scope allocation budgetből. Unicode case conversionnél konzervatív felső becslést használunk; `replace`, `split` és `repeat` esetén a kimeneti méretet előre számoljuk vagy konzervatívan becsüljük.

A műveletek külön instruction költséget is kapnak, tehát nem jelentenek budget nélküli kerülőutat.

Futtatható példák: `examples/string-operations/main.vrn` és `examples/string-list/main.vrn`.

## Statement szintaxis

A példák `let` sorai egyszerű statementek, ezért explicit `;` zárja őket; a sortörés önmagában nem terminátor. Lásd: [Statement terminátorok](55-statement-terminatorok.md). A `String + String` konkatenáció és az operátorprecedencia részletesen: [Numerikus operátorok, f32 matematika és monoton időmérés](43-matematika-es-idomeres.md).
