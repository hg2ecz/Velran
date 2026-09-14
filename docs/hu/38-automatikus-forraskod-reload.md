<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Automatikus alkalmazás-forráskód reload

A Velran a frissen feltöltött `.vrn` alkalmazáskódot process restart nélkül is érvényre tudja juttatni. A source reload tranzakciós: a megváltozott alkalmazásból először külön candidate runtime készül, és az élő domain csak sikeres fordítás és validáció után vált át rá. Hibás új kód esetén az előző működő generáció szolgál tovább.

## Konfiguráció

Globális alapértékek:

```toml
[reload]
enabled = true
poll_interval_ms = 1000
debounce_ms = 250
debug_compile_errors = false
```

Domainenként felülírható:

```toml
[[domains]]
host = "example.com"
workdir = "/srv/velran/domains/example.com/current"
app = "main.vrn"

[domains.reload]
enabled = true
poll_interval_ms = 1500
debounce_ms = 300
```

Karbantartáskor kikapcsolható globálisan `reload.enabled = false`, egy domainre `domains.reload.enabled = false`, vagy az egész processre a `--no-source-reload` kapcsolóval.

## Mit figyel a szerver?

A reload supervisor a konfigurált alkalmazás entrypoint könyvtára alatti `.vrn` source tree stabil snapshotját figyeli. Ezért egy újonnan bemásolt, átnevezett vagy törölt `.vrn` fájl is automatikusan candidate buildet indíthat; nem szükséges, hogy az előző sikeres fordítás dependency listájában már szerepeljen. A watcher nem hash-el source-ot HTTP requestenként: olcsó fájlrendszer-metaadatból és debounce-olt snapshotból dolgozik, a teljes fordítás csak változás után indul.

A canonical modulútvonalak mellett a konfigurált logikai `app` útvonal is figyelt. Ez az atomikus release-mintánál fontos:

```text
/srv/velran/domains/example.com/current -> releases/2026-09-05.2
```

A `current` symlink átállítása ezért akkor is új fordítást indít, ha a korábbi dependency graph canonical útvonalai még az előző release könyvtárába mutattak.

Az összes domaint egyetlen közös supervisor kezeli, nem domainenként külön polling thread. Minden domain a saját effektív `poll_interval_ms` értéke szerint kerül ellenőrzésre.

## Változás, debounce, fordítás, commit

Forrásváltozás észlelése után a Velran `debounce_ms` ideig stabil fájlállapotra vár. Így egy többfájlos feltöltés nem indít minden egyes fájl után külön fordítást.

Ezután külön candidate alkalmazás fordul és ugyanazokon a hosting-validációkon megy át, mint startup/reload során. Siker esetén csak az adott domain runtime-ja cserélődik atomikusan. A régi runtime-ot már használó requestek azon fejeződnek be, az új requestek pedig az új generációt kapják.

Sikeres source reload előtt a domain public-cache route generationjei is előrelépnek. Emiatt a régi kód által generált HTML/JSON nem marad látható csak azért, mert a korábbi TTL még nem járt le.

## Új, törölt vagy késve feltöltött almodul

Új modul természetesen követhető: az új `mod` deklaráció miatt a már ismert parent source megváltozik, ez fordítást indít. Siker után a compiler új dependency graphot ad vissza, amely már tartalmazza az új modult is.

Figyelt modul törlése vagy átnevezése szintén változás. A candidate fordítás hibázik, a hiba logolódik, a korábbi működő generáció pedig aktív marad.

Több fájl feltöltésekor előfordulhat, hogy a parent modul már hivatkozik egy új child fájlra, de az még nem érkezett meg. Ilyenkor az első fordítás jogosan elbukhat. A Velran a stabil, sikertelen candidate-et exponenciális backoffal újrapróbálja: 2 másodperctől indul, legfeljebb 60 másodpercig ritkul. Így a később megérkező modul külön kézi reload nélkül is életbe léphet, miközben egy tartósan hibás forrás nem okoz folyamatos újrafordítást.

## Hiba és naplózás

A sikertelen automatikus reload **nem állítja le a domaint**. Az előző valid generáció fut tovább. Fontos strukturált logesemények:

- `source_change_detected`
- `source_reload_committed`
- `source_reload_rejected`
- `source_reload_cache_invalidation_failed`
- `source_reload_stale`

A `source_reload_rejected` tartalmazza a canonical domaint, az aktív generationt és a compiler/validációs hibát, ezért a szintaktikai és modulhibák diagnosztizálhatók a server logból.

## Ajánlott deployment

Normál, csak alkalmazáskódot érintő kiadásnál:

1. töltsd fel/építsd fel az új immutable release könyvtárat;
2. szükség esetén futtasd az `velran-server --config ... --check-config` preflightot;
3. atomikusan állítsd át a domain `current` symlinkjét, vagy frissítsd a figyelt forrásokat;
4. a source-reload supervisor lefordítja és siker esetén commitolja az új generációt;
5. ellenőrizd a `source_reload_committed` eseményt, a health/readiness állapotot és a smoke tesztet.

A listener, DB/Redis/auth kapcsolat, cgroup limit, logging sink és más process-szintű beállítás továbbra is a normál config/restart lifecycle része. Behind-proxy módban a `SIGHUP` a domain/application konfiguráció tranzakciós reloadjára szolgál; az automatikus source reload ennél könnyebb, kifejezetten alkalmazáskódra szánt út.

## Fejlesztői compiler hibaképernyő a terminálon

Scripting jellegű fejlesztéshez kérhető részletes fordítási hiba:

```sh
velran-server --debug-compile-errors ...
```

vagy konfigurációból:

```toml
[reload]
debug_compile_errors = true
```

Sikertelen candidate esetén az utolsó valid generation tovább szolgál. A terminálon stabil hibakód/security-kód, relatív source path, sorszám, korlátozott forrássor és – ahol rendelkezésre áll – javítási segítség jelenik meg. A renderer control karaktereket eltávolít és hosszkorlátot alkalmaz, ezért source tartalom nem használható terminál escape-injectionre. A `debug_disabled` production policy ezt a módot elutasítja.
