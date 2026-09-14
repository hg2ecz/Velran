<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 16. Üzemeltetői fejezet

Ez a fejezet az operator feladatait foglalja össze. Az alkalmazáskód ezeket a policy-ket nem írhatja felül.

## Felelősségek

Az operator birtokolja:

```text
TLS private key
DB/Redis/LDAP credential
resource profile config
AppFs data_root
cgroup/systemd policy
egress policy
trusted proxy list
```

## Production minimum

- dedikált unprivileged OS user;
- program/config read-only;
- secret files külön, szigorú permissionnel;
- writable csak AppFs `data_root`;
- HTTPS és explicit `tls.public_host`;
- DB/Redis/LDAP TLS;
- Redis production session backend clusterhez;
- request soft limit + cgroup hard limit;
- backup, restore-próba és rollback artifact.

## DB

Application credential csak szükséges DML jogokat kapjon. Migráció külön credential. Poolméretezésnél:

```text
összes DB connection ≈ instance_count × per_instance_pool
```

Hagyj tartalékot admin/maintenance kapcsolatokra.

## Redis

Ne legyen automatikus in-memory fallback production kieséskor. Monitorozd latency-t, reconnectet, memory usage-t és evictiont.

## Resource profile-ok

A forrás csak statikus nevet kérhet, például `compute`. Az operator configolja a számokat és `max_concurrent` értéket.

Startupkor minden nem-default profile-hely egyszer auditálódjon: forrásfájl, sor, függvény, profil és feloldott limit. Ismeretlen vagy policyt sértő profil esetén fail-closed startup.

A profile nem lépheti át a request hard ceilingt vagy a cgroup/RLIMIT plafont.

## Monitoring/audit

Külön kezeld:

- access log;
- security audit;
- startup configuration/resource-profile audit;
- metrics.

Ne logolj Authorization/Cookie headert, session ID-t, CSRF/TOTP secretet, DB/Redis credentialet vagy secret value-t.

## Release

Deploy előtt:

```bash
./verify.sh
cargo check --workspace
cargo test --workspace
```

és futtasd valós PostgreSQL/MariaDB/Redis/TLS/openat2/cgroup környezetben a release checklist releváns pontjait.

## Resource profile startup audit

Productionban a named profile configot trusted, read-only fájlból hivatkozd:

```toml
[limits]
resource_profiles_file = "/usr/local/etc/velran/resource-profiles.toml"
```

Az induláskori `resource_profile_use` eseményeket érdemes deployment artifactként megőrizni. Így auditálható, melyik forrásfájl és függvény kér a defaultnál nagyobb compute/memória soft keretet. Ismeretlen profil startup error.

## CORS

CORS allowlist deployment policy. Csak a tényleges frontend origineket add meg a trusted configban:

```toml
[web]
cors_origins = ["https://frontend.example"]
cors_allow_credentials = true
```

Credentialed CORS csak HTTPS-en használható; ilyenkor a session cookie `SameSite=None; Secure`. A CORS policy változása security-review tárgya.

## Static asset deploy

A `[static_assets].root` read-only deployment artifact legyen. Fingerprintelt fájlneveket használj, és opcionálisan build időben készíts `.br`/`.gz` siblingeket. Ne irányítsd a static rootot az upload/data könyvtárra.

## Health és shutdown

Load balancer/Kubernetes probe:

```text
liveness  GET /health/live
readiness GET /health/ready
```

A readiness DB/Redis kiesésnél `503`, így az instance kivehető a forgalomból. A probe-ot ne tedd publikus üzleti monitoring API-vá; csak minimális státuszt ad.

Rolling restartnál SIGTERM-et használj. Az orchestrator termination grace periodja legyen nagyobb a `lifecycle.shutdown_grace_ms` configértéknél, hogy a Velran előbb le tudja drainelni az aktív kapcsolatokat.

## Route rate limiting

Productionban a route policy config read-only trusted fájl legyen, és Redis-backed limiter fusson. A `[rate_limit].allow_memory = false` maradjon production default; memory limiter csak fejlesztési kivétel. Monitorozd a 429 és `rate_limiter_unavailable` eseményeket policy/route alacsony cardinality labellel; raw IP/user ne legyen metric label.

## Observability

Prometheus scrape-hez külön management listen címet használj a trusted configban:

```toml
[observability]
metrics_listen = "127.0.0.1:9090"
```

Ha nem-loopback bind szükséges, az explicit public-metrics engedélyezés mellett hálózati ACL/reverse proxy auth is kell. A megfelelő CLI opciók célzott override-k, nem ajánlott production policy surface-ek. A structured access log JSON lines formátumú; log collectorral stdout/stderr-ről gyűjthető. Secret és request body nem kerülhet logba.

## Migration deployment

A schema migráció külön deploy művelet és külön credential. Ajánlott sorrend: `migrate verify` → `migrate apply` → `migrate verify` → application rollout. Az alkalmazásszerver ne kapja meg a migration credentialet. Részletek: [Database migration workflow](10-database-migrations.md).

## Local auth SQLite

Az auth DB külön secret/security boundary az application DB-től. A DB URL file és maga az SQLite DB csak a service/operator user számára legyen olvasható (`0600`/szűk parent directory). Backupold titkos adatként: password hash, TOTP secret és recovery hash található benne. Clusterhez M25-ben LDAP ajánlott shared identity backendként.

## Public cache

Cached public routes productionban Redis-backed `velran-cache` namespace-t használnak. Az operator állítja a maximális TTL-t és a memory fallback korlátait. A `[cache].allow_memory = false` maradjon productionban; memory fallback csak development kivétel. Cache hit/miss a Prometheus exportban külön számláló.

## Server config

Productionban a stabil beállításokat trusted, read-only `/usr/local/etc/velran/server.toml` fájlban tartsd. Kiindulópont: `config/server.toml.sample`. Deploy előtt:

```bash
velran-server --config /usr/local/etc/velran/server.toml --check-config
```

A secretek külön `/run/secrets/...` fájlok maradjanak.

## Logrotate

Ajánlott három külön file: server/access/audit. A log directory legyen az operator tulajdona, a service user csak a szükséges írási jogot kapja.

A projektben található minta: `examples/logrotate/velran`. Rotation után SIGHUP-ot küld; a Velran ekkor **csak a log file descriptorokat** nyitja újra.

```bash
systemctl reload velran.service
```

A systemd sample `ExecReload=/bin/kill -HUP $MAINPID` beállítást tartalmaz. Configváltozáshoz ne reloadot, hanem ellenőrzött restartot használj:

```bash
/usr/local/bin/velran-server --config /usr/local/etc/velran/server.toml --check-config
systemctl restart velran.service
```

## Logging & audit baseline

Az üzemeltető külön kezelje a system/error, access és user/security audit naplókat. Az audit retention legyen dokumentált szervezeti döntés; a helyi audit fájlt ajánlott centralizált, hozzáférés-védett log/SIEM tárba továbbítani. Monitorozd a `velran_log_fallback_total` számlálót: növekedése log-processing hibát vagy queue nyomást jelez. Az időszinkronizáció host/orchestrator feladat; a naplók UTC timestampet használnak.


## M45 ajánlott deployment út

Új production telepítésnél a `examples/starter-project/` és a [Production deployment és starter project](29-production-deployment.md) legyen a referencia. A service indítása config-first; a régi, hosszú CLI-flaglistás systemd minta nem ajánlott.

Ajánlott sorrend: immutable artifact + hash → backup/restore readiness → preflight → külön migration apply → atomic release switch → controlled restart → live/ready → log/metric/audit review.

## Backup, restore, upgrade és rollback

M46-tól a production recovery referencia a [Backup, restore, upgrade és rollback](30-backup-restore-upgrade-rollback.md). A V1 safe baseline controlled maintenance/offline snapshot, mert az application DB és AppFs együtt alkothat üzleti állapotot. Local-auth SQLite külön secret backup. Redis session/cache/rate-limit állapot nem business source of truth; disaster recovery után session reset és cache rebuild elfogadott.

Application rollback csak akkor legyen egy egyszerű previous-release switch, ha a schema backward-compatible. Automatikus reverse migration nincs. Destruktív schema/data visszaállítás explicit recovery művelet, választott recovery pointtal és dokumentált adatvesztési kockázattal.
