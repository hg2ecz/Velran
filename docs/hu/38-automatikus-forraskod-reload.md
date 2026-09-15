<!-- VELRAN-DOC-STATUS: 2026-09-15 -->
> **Dokumentációs státusz (2026-09-15):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Automatikus alkalmazás-forráskód reload

A Velran a frissen feltöltött `.vrn` alkalmazáskódot process restart nélkül is érvényre tudja juttatni. A source reload tranzakciós: a megváltozott alkalmazásból először külön candidate runtime készül, és az élő domain csak sikeres fordítás és validáció után vált át rá. Hibás új kód esetén az előző működő generáció szolgál tovább.

## Konfiguráció

Globális alapértékek:

```toml
[reload]
enabled = true
mode = "development"
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
mode = "development"
poll_interval_ms = 1500
debounce_ms = 300
```

Karbantartáskor kikapcsolható globálisan `reload.enabled = false`, egy domainre `domains.reload.enabled = false`, vagy az egész processre a `--no-source-reload` kapcsolóval.

## Mit figyel a szerver?

A reload supervisor a konfigurált alkalmazás forrásgyökereiben a releváns `.vrn` és `velran.toml` fájlokat metaadattal és SHA-256 tartalom-fingerprinttel figyeli. Ezért azonos méretű szerkesztés, újonnan bemásolt, átnevezett vagy törölt forrás is észlelhető. A fingerprintelés csak a polling/debounce útvonalon történik, HTTP requestenként nem.

A canonical modulútvonalak mellett a konfigurált logikai `app` útvonal is figyelt. Ez az atomikus release-mintánál fontos:

```text
/srv/velran/domains/example.com/current -> releases/2026-09-05.2
```

A `current` symlink átállítása ezért akkor is új fordítást indít, ha a korábbi dependency graph canonical útvonalai még az előző release könyvtárába mutattak.

Az összes domaint egyetlen közös supervisor kezeli, nem domainenként külön polling thread. Minden domain a saját effektív `poll_interval_ms` értéke szerint kerül ellenőrzésre.

## Változás, debounce, fordítás, commit

Forrásváltozás észlelése után a Velran `debounce_ms` ideig stabil fingerprintre vár. Így egy többfájlos feltöltés nem indít minden egyes fájl után külön fordítást.

Ezután külön candidate alkalmazás fordul és ugyanazokon a hosting-validációkon megy át, mint startup/reload során. Aktiválás előtt a Velran újra fingerprinteli az élő forrásfát; ha a build közben bármi változott, a candidate eldobódik és új debounce/build kör indul. Csak stabil candidate esetén cserélődik atomikusan az adott domain runtime-ja. A régi runtime-ot már használó requestek azon fejeződnek be, az új requestek pedig az új generációt kapják.

Sikeres source reload előtt a domain public-cache route generationjei is előrelépnek. Emiatt a régi kód által generált HTML/JSON nem marad látható csak azért, mert a korábbi TTL még nem járt le.

## Új, törölt vagy késve feltöltött almodul

Új modul természetesen követhető: az új `mod` deklaráció miatt a már ismert parent source megváltozik, ez fordítást indít. Siker után a compiler új dependency graphot ad vissza, amely már tartalmazza az új modult is.

Figyelt modul törlése vagy átnevezése szintén változás. Ha a modult a rá mutató hivatkozásokkal és route-okkal együtt konzisztensen kivonják a projektből, a candidate sikeresen lefordulhat, és a törölt modul, handler illetve route már nem része az újonnan aktivált generationnek. Ha viszont bármely élő forrás továbbra is a hiányzó modulra vagy szimbólumra hivatkozik, a candidate validációja elbukik, a hiba logolódik, és a korábbi működő generation marad aktív. A törlés tehát tranzakciós: vagy egy teljes, érvényes generation részeként lép életbe, vagy nincs hatása az élő forgalomra.

Több fájl feltöltésekor előfordulhat, hogy a parent modul már hivatkozik egy új child fájlra, de az még nem érkezett meg. Ilyenkor az első fordítás jogosan elbukhat. A Velran a stabil, sikertelen candidate-et exponenciális backoffal újrapróbálja: 2 másodperctől indul, legfeljebb 60 másodpercig ritkul. Így a később megérkező modul külön kézi reload nélkül is életbe léphet, miközben egy tartósan hibás forrás nem okoz folyamatos újrafordítást.

## Hiba és naplózás

A sikertelen automatikus reload **nem állítja le a domaint**. Az előző valid generáció fut tovább. Fontos strukturált logesemények:

- `source_change_detected`
- `reload_candidate_started`
- `reload_candidate_ready`
- `reload_candidate_source_changed`
- `reload_activated`
- `reload_previous_generation_retained`
- `source_reload_rejected`
- `source_reload_cache_invalidation_failed`
- `source_reload_stale`

A `source_reload_rejected` tartalmazza a canonical domaint, az aktív generationt és a compiler/validációs hibát, ezért a szintaktikai és modulhibák diagnosztizálhatók a server logból.

## Ajánlott deployment

A Velran két alkalmazáskód-deployment stílust támogat. Rolling production módban a webfejlesztő közvetlenül feltöltheti vagy szerkesztheti a figyelt `.vrn` fájlokat; a debounce, a stabil forrás-fingerprint, a candidate validáció és az atomikus aktiválás védi az élő generationt. Kontrollált kiadásnál továbbra is használható immutable release könyvtár és atomikus `current` symlink, de ez nem követelmény.

Ajánlott rolling folyamat:

1. töltsd fel a módosított `.vrn` fájlokat a figyelt alkalmazásfába;
2. hagyd, hogy a source-reload supervisor stabil fingerprintre várjon és felépítse a candidate-et;
3. ellenőrizd a `reload_activated` eseményt, a health/readiness állapotot és a smoke tesztet.

Kontrollált immutable release esetén opcionálisan fusson `velran-server --config ... --check-config`, majd atomikus `current` symlink-váltás, végül ugyanaz az aktiválási és smoke-check ellenőrzés.

A listener, DB/Redis/auth kapcsolat, cgroup limit, logging sink és más process-szintű beállítás továbbra is a normál config/restart lifecycle része. Behind-proxy módban a `SIGHUP` a domain/application konfiguráció tranzakciós reloadjára szolgál; az automatikus source reload ennél könnyebb, kifejezetten alkalmazáskódra szánt út.

## Rolling production mód

PHP-szerű közvetlen fájlfeltöltéshez:

```toml
[reload]
enabled = true
mode = "rolling"
poll_interval_ms = 1000
debounce_ms = 1000
debug_compile_errors = false
```

A `rolling` a production-safe reload mód. A szigorú `production { debug disabled; ... }` policy ezt engedi, mert az aktiválás tranzakciós és last-known-good szemantikájú. Ugyanez a policy továbbra is tiltja a `mode = "development"` reloadot, a részletes compiler-hibaoldalt, az insecure development cookie-kat és a `native.debug_rustc_repro = true` beállítást. `native.required = true` mellett az első startup továbbra is fail-closed; későbbi rolling candidate hiba viszont csak az új candidate-et utasítja el, a jelenlegi generation aktív marad. Teljes minta: `config/server-rolling-prod.toml.sample`.

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
