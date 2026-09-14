<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Production deployment és starter project

A Velran production ajánlása egyetlen, auditálható út: **immutable release + trusted TOML config + külön secret files + explicit migration step + controlled restart**.

Kiindulópont: `examples/starter-project/`.

## 1. Projektstruktúra

```text
main.vrn
models.vrn
queries.vrn
pages.vrn
actions.vrn
migrations/
public/
```

A `main.vrn` csak a modulgráf entrypointja. Runtime adat, upload és secret ne kerüljön a source/release könyvtárba.

## 2. Release build és ellenőrzés

```bash
./verify.sh
cargo build --locked --release -p velran-server -p velran-cli
sha256sum target/release/velran-server target/release/velran-cli
```

A release record tartalmazza legalább az alkalmazás artifact azonosítóját, SHA-256-át, migration setet és a jóváhagyott config verzióját. Secret value-t ne rögzíts.

## 3. Immutable release layout

```text
/srv/velran/releases/2026-09-04.1/
/srv/velran/current -> /srv/velran/releases/2026-09-04.1
/srv/velran/data/
/usr/local/etc/velran/server.toml
/run/secrets/velran/
/var/log/velran/
```

A `releases/*` és `/usr/local/etc/velran` legyen runtime read-only. A service csak a deklarált data/log pathokra írjon.

## 4. Config és secret

Productionban:

```bash
velran-server --config /usr/local/etc/velran/server.toml --check-config
```

A stabil limitek, listener, TLS, health, log, cache, rate-limit és resource-profile policy a TOML/config fájlokban maradjon. Jelszó, DB/Redis URL vagy private key ne legyen CLI argument vagy plaintext TOML value; használj `*_file` beállítást.

## 5. Preflight

A starter `deploy/preflight.sh` három fail-fast ellenőrzést futtat:

```text
server config validation
→ application source compile/check
→ migration verify
```

A script szándékosan **nem** futtat `migrate apply`-t és nem készít backupot. Ezek operator által jóváhagyott állapotváltoztató lépések.

## 6. Migration és rollout

Ajánlott sorrend:

```text
backup + restore readiness
→ migrate verify
→ migrate apply
→ migrate verify
→ new release install
→ atomic current switch
→ systemctl restart velran
→ /health/live
→ /health/ready
→ log/metric/audit review
```

A migration credential csak a migration CLI folyamat számára legyen elérhető. Az application runtime credential maradjon least-privilege DML credential.

## 7. systemd

Minta: `examples/starter-project/deploy/velran.service` és `examples/systemd/velran.service`.

Az `ExecStart` config-first:

```text
/usr/local/bin/velran-server --config /usr/local/etc/velran/server.toml
```

Nem ismételjük meg több tucat CLI flagben ugyanazt a production policyt. `ExecStartPre` futtatja a `--check-config` ellenőrzést. A service unprivileged user alatt fut, és a systemd unit adja a hard process/cgroup limiteket. Emiatt a párosított `server.toml` ne írjon ugyanabba a cgroup policyba; külön Velran `[cgroup]` csak explicit delegált, nem-systemd authority esetén kell.

## 8. Health és forgalomba állítás

```text
GET /health/live
GET /health/ready
```

A rollout csak readiness siker után tekinthető késznek. DB/Redis dependency hiba esetén a readiness 503; ezt a load balancer/orchestrator használja, ne üzleti monitoring API.

## 9. Logging és rotation

Külön server, access és audit log. Rotation után:

```bash
systemctl reload velran.service
```

Behind-proxy módban a SIGHUP log-reopen mellett tranzakciósan újraolvassa a domain/application hosting állapotot. Process-szintű config változásra:

```bash
velran-server --config /usr/local/etc/velran/server.toml --check-config
systemctl restart velran.service
```

## 10. Rollback határ M45-ben

Az immutable application release atomikusan visszaállítható korábbi artifactra, **ha az adatbázis-séma kompatibilis**. Ne futtass automatikus reverse SQL-t csak azért, mert az alkalmazást visszaállítod. A teljes backup/restore, forward-compatible migration, rollback és upgrade policy M46 témája.

## Release exit criteria

- `./verify.sh` zöld;
- artifact hash rögzítve;
- config check zöld;
- migration verify zöld;
- backup/restore readiness igazolt;
- rollout után live + ready zöld;
- nincs startup error vagy secret leak a logban;
- security audit és resource-profile startup audit átnézve.

## 11. M46 recovery contract

Az M45 `backup + restore readiness` gate részletes definíciója: [Backup, restore, upgrade és rollback](30-backup-restore-upgrade-rollback.md).

Röviden: a safe V1 baseline controlled stop/drain után készített application DB + local-auth DB + AppFs mentés, secretmentes manifesttel és rendszeresen bizonyított restore-drillel. App-only rollback csak backward-compatible migration esetén ajánlott; destructive recovery nem azonos a release symlink visszaállításával.


## Automatikus alkalmazáskód-élesítés

Normál `.vrn` release-nél a szervernek nem kell újraindulnia. A közös source-reload supervisor az entrypointot és a compiler által visszaadott teljes tranzitív modulgráfot figyeli `mtime + size` alapján. Változás után debounce következik, majd candidate fordítás és validáció. Csak sikeres candidate kerül atomikusan az élő domain helyére; hibás feltöltésnél a korábbi generáció szolgál tovább.

Az ajánlott release-layout immutable könyvtár + atomikusan cserélt `current` symlink. A logikai `app` útvonal figyelése miatt a symlink-csere is reloadot indít. Sikeres commitkor a domain public-cache generationjei is invalidálódnak, így a régi generált tartalom nem marad a korábbi TTL végéig.

Konfiguráció és részletes hibaviselkedés: [Automatikus alkalmazás-forráskód reload](38-automatikus-forraskod-reload.md).
