<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Típusos string dictionary

A Velran `BTreeMap<String,String>` compute-local kollekciót ad kis, determinisztikus kulcs-érték táblákhoz, például metaadatokhoz, feldolgozott inputhoz és lookup táblákhoz.

```vrn
let values = BTreeMap::new();
values["name"] = "Velran";
values["mode"] = "production";

let count = values.len();
let exists = values.contains_key("name");
let name = values["name"];

values = removeKey(values, "mode");
```

A kulcs és az érték is `String`; nincs implicit konverzió más skalár típusból.

## Hiányzó kulcs

A `values["missing"]` szigorú: hibát ad, nem üres stringet. Ha a kulcs hiánya normális eset, előbb `values.contains_key(key)` használható.

## Módosítás

Beszúrás és felülírás:

```vrn
values[key] = value;
```

A `removeKey(dict, key)` új dictionary értéket ad vissza, ezért törléskor:

```vrn
values = removeKey(values, key);
```

## Limitek

Legfeljebb 4096 bejegyzés lehet. A kulcs nem lehet üres, és legfeljebb 1024 UTF-8 bájt lehet. A kulcsok, értékek és a kollekció overheadje beleszámít a runtime allocation budgetbe.

A `BTreeMap<String,String>` jelenleg compute-local típus: nem model mező, DB skalár, route/form inputtípus és nem business-audit skalár.
