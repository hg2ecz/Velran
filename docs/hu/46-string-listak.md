<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Típusos String listák

A Velran most már rendelkezik compute-local `Vec<String>` értékkel. Az első létrehozó művelet a `splitBounded(text, delimiter, maxItems)`.

```vrn
let parts = splitBounded("alpha,beta,gamma", ",", 16);
let count = parts.len();
let first = parts[0];
```

A `split` típusa:

```text
splitBounded(String, String, i64) -> Vec<String>
```

A szeparátor nem lehet üres. A `split` legfeljebb 4096 elemet hozhat létre, és az eredményül kapott stringek, valamint a kollekció overheadje beleszámít a runtime memóriafoglalási keretébe.

A `Vec<String>` egyelőre compute-local típus. Nem használható model mezőként, route/form skalárként, adatbázis skalárként vagy audit skalárként. Így a perzisztencia és a request schema explicit marad, amíg az általános collection réteg tovább épül.

Az indexelés futásidőben bounds-checkelt. Negatív vagy túl nagy index fail-closed hibát ad, nem implicit null értéket.

A collection expression réteg közös a korlátos `f32` tömbökkel:

```vrn
let samples = vec![0.0f32; 4096];
let n = samples.len();
let x = samples[0];

let words = splitBounded("one two three", " ", 16);
let m = words.len();
let word = words[0];
```

Ez a közös Rust-szerű `collection.len()` és `collection[index]` reprezentáció a későbbi típusos kollekciók és dictionary alapja.

Futtatható példa: `examples/string-list/main.vrn`.
