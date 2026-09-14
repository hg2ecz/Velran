<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Reguláris kifejezések

A Velran kis, erőforrás-korlátos regexp API-t ad kérésfeldolgozáshoz és szövegnormalizáláshoz.

```vrn
let ok = regexMatch(text, "^[A-Z]{2}-[0-9]{4}$");
let normalized = regexReplace(text, "[^A-Za-z0-9]+", "-");
let captures = regexCaptures(text, "^(?P<name>[a-z]+)-(?P<id>[0-9]+)$");
```

A függvények szigorúan típusosak:

```text
regexMatch(String, String) -> bool
regexReplace(String, String, String) -> String
regexCaptures(String, String) -> BTreeMap<String,String>
```

A `regexCaptures` nem találat esetén üres dictionaryt ad. Találatkor a capture-ök számozott kulccsal (`"0"`, `"1"`, ...) és névvel is elérhetők. A nem illeszkedő opcionális csoport kimarad; ilyen esetben indexelés előtt `containsKey` használata javasolt.

## Biztonság és limitek

A runtime a Rust `regex` motorját használja, amely nem támogat olyan backreference/look-around konstrukciókat, amelyek korlátlan backtrackinget igényelnének. A Velran ezen felül explicit limiteket tart:

- pattern: legfeljebb 4096 UTF-8 bájt;
- input: legfeljebb 1 MiB;
- replacement template: legfeljebb 16 KiB;
- legfeljebb 64 capture csoport;
- generált replacement/capture adat: legfeljebb 16 MiB, és továbbra is beleszámít a normál runtime allocation budgetbe.

Hibás regexp vagy limitátlépés request hibát ad, nem panicot. A regexp műveletek instruction-költsége magasabb az egyszerű skalárműveletekénél.

A lefordított regexp objektum szándékosan nem Velran érték. Ez tisztán tartja a nyelvi modellt, és később lehetőséget ad korlátos compiled-pattern cache bevezetésére a forrásnyelv megváltoztatása nélkül.
