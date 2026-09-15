<!-- VELRAN-DOC-STATUS: 2026-09-15 -->
> **Dokumentációs státusz (2026-09-15):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Velran starter project

Ez a könyvtár M45 production-közeli kiindulópont. Nem demo deployment defaults: a hostneveket, secreteket, limiteket és OS pathokat operatornak kell beállítania.

## Fejlesztés

```bash
cargo run --locked -q -p velran-cli -- check examples/starter-project/main.vrn
```

A projekt `main.vrn` entrypointból és explicit deklarált, névtérbe rendezett forrásfájlokból áll. A `mod models;` a `models.vrn` fájlt a `models` névtérként tölti be; az `Article` és más deklarációk nem kerülnek globális scope-ba. Modulhatáron át ezért kvalifikált név kell, például `models::Article` vagy `queries::articleBySlug(...)`. A migration külön deploy művelet; az alkalmazásszerver nem futtat automatikus schema migrationt.

## Release artifact layout

Ajánlott immutable release könyvtár:

```text
/srv/velran/releases/2026-09-04.1/
  main.vrn
  models.vrn
  queries.vrn
  pages.vrn
  actions.vrn
  migrations/
  public/
/srv/velran/current -> /srv/velran/releases/2026-09-04.1
```

A release könyvtár read-only. Írható adat kizárólag a külön `/srv/velran/data` alatt legyen.

## A mellékelt service/config authority modellje

A starter `deploy/server.toml` közvetlen TLS-t mutat 80/443 porton, miközben a process `velran` userként fut. Ezért a párosított systemd unit csak a `CAP_NET_BIND_SERVICE` capabilityt adja a service-nek; root futtatás nem szükséges. Ha Apache/Nginx mögött `127.0.0.1:8080` listenert használsz, ezt a capabilityt távolítsd el.

A process cgroup hard limitjeinek authorityja ebben a starterben **systemd** (`MemoryMax`, `MemorySwapMax`, `CPUQuota`, `TasksMax`). Emiatt a `server.toml` nem kapcsolja be az Velran `[cgroup]` író módját. A két cgroup authorityt ne használd egyszerre.

## Production sorrend

1. build + `./verify.sh`;
2. artifact SHA-256 rögzítése;
3. adatbázis-backup és tényleges restore-readiness ellenőrzése;
4. `deploy/preflight.sh`;
5. `migrate apply` külön migration credentialdel;
6. új immutable release telepítése;
7. `current` symlink atomikus átállítása;

PHP-szerű, közvetlen feltöltéses oldalnál a symlink folyamat opcionális: állítsd `reload.mode = "rolling"` értékre, és töltsd fel közvetlenül a figyelt `.vrn` fájlokat. A Velran stabilizálja, validálja és atomikusan aktiválja a candidate-et; hibás vagy félkész feltöltésnél a korábbi generation marad aktív. Konzisztens forrástörlés a következő sikeres aktiváláskor visszavonja a törölt route/modul kiszolgálását.
8. controlled `systemctl restart velran`;
9. `/health/live` és `/health/ready` ellenőrzése;
10. log/metrics/audit ellenőrzése.

Rollbacknél az alkalmazás artifact visszaállítható korábbira, de schema rollbacket nem szabad automatikusan feltételezni. A DB kompatibilitást előre kell megtervezni; részletes backup/restore/upgrade/rollback szerződés M46 feladata.

## M46 recovery

Részletes operator contract: `../../docs/hu/30-backup-restore-upgrade-rollback.md`.

A starter `deploy/release-record.sh` secretmentes release/config/migration hash rekordot készít. A `deploy/restore-verify.sh` csak izolált restore-test környezetben, read-only módon futtat config/source/migration ellenőrzést; production pathokra fail-closed módon megtagadja a futást.

A V1 safe backup baseline controlled service stop/drain után application DB + local-auth DB + teljes AppFs data root mentés. Redis session/cache/rate-limit állapot restore-ja nem szükséges; session reset elfogadott.
