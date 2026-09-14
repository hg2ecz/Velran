<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Backup, restore, upgrade és rollback

A Velran V1 operator contract célja nem egy mágikus `backup` parancs, hanem egy **bizonyíthatóan konzisztens és visszaállítható állapot**. A backup akkor tekinthető használhatónak, ha restore-próbából induló alkalmazás `migrate verify`, source/config check és health ellenőrzése is sikeres.

## 1. Mit kell menteni?

Production állapot legalább:

```text
application database
local-auth SQLite database, ha local auth használatban van
AppFs data_root (upload/media/application data)
immutable release azonosító + artifact hash
migration set + checksumok
trusted config/policy fájlok verziója vagy hash-e
backup időpont + alkalmazásverzió + DB backend
```

A secret fájlok kezelése szervezeti secret-management feladat. Ne másold őket plaintext release-manifestbe vagy logba. A recovery eljárásnak viszont dokumentálnia kell, honnan állíthatók helyre a szükséges DB/Redis/TLS/LDAP secretek.

### Redis

A jelenlegi Velran Redis használata session, rate-limit és public-cache állapot. Ezek **nem üzleti source of truth**. Disaster restore után elfogadható, sőt biztonságos alapértelmezés az üres Redis: a sessionök megszűnnek, a cache újraépül, a limiter ablakai újraindulnak.

Ha az operator ugyanazt a Redis clustert más alkalmazás üzleti adataihoz is használja, annak backupja már nem Velran-specifikus contract.

## 2. Konzisztencia: az ajánlott V1 baseline

A DB és az AppFs együtt alkothat üzleti állapotot. Például egy DB sor hivatkozhat feltöltött képre. Emiatt külön időpontban készített DB dump + data-root tar nem állítható automatikusan konzisztens snapshotnak.

**V1 safe default:** controlled maintenance/offline backup.

```text
forgalom kivétele
→ SIGTERM / graceful drain
→ service stopped állapot igazolása
→ application DB backup
→ local-auth DB backup
→ AppFs data_root snapshot
→ manifest + hash
→ service indítás
→ readiness
```

Ez rövid írási szünetet okozhat, viszont egyszerűen auditálható.

Online backup csak akkor nevezhető konzisztensnek, ha a választott DB/storage infrastruktúra ugyanahhoz a recovery ponthoz kötött snapshotot biztosít, vagy az alkalmazás-specifikus adatmodell bizonyíthatóan tolerálja a komponensek eltérő snapshotidejét. A Velran runtime önmagában nem ígér distributed snapshotot.

## 3. Backend-specifikus DB backup

A Velran nem rejti el a natív adatbázis backup eszközeit. Az operator a backend támogatott eszközével készít mentést, külön backup credentialdel.

### PostgreSQL

Ajánlott logikai backup:

```bash
pg_dump --format=custom --no-owner --no-acl --file app.dump "$DATABASE_URL"
```

Restore izolált adatbázisba:

```bash
createdb velran_restore_test
pg_restore --no-owner --no-acl --dbname velran_restore_test app.dump
```

A production secretet ne tedd shell historyba. A fenti `$DATABASE_URL` csak szemléltetés; productionban használj a PostgreSQL kliens által támogatott védett credential-forrást.

Nagy adatbázisnál fizikai backup/PITR is lehet jobb, de annak recovery runbookja a DB platform része.

### MariaDB

Logikai backup tipikus InnoDB alkalmazásnál:

```bash
mariadb-dump --single-transaction --routines --triggers app > app.sql
```

Restore izolált adatbázisba:

```bash
mariadb velran_restore_test < app.sql
```

`--single-transaction` csak tranzakcionális tábláknál ad konzisztens logikai képet. Vegyes/non-transactional schema esetén a maintenance/offline baseline maradjon az alap.

### SQLite application DB

Ne másolj élő SQLite DB fájlt egyszerű `cp`-vel. Használd a SQLite online backup mechanizmusát vagy stopped állapotban fájlrendszer-snapshotot.

CLI példa:

```bash
sqlite3 /srv/velran/data/app.db ".backup '/backup/app.db'"
```

Restore-próbához másold külön test pathra, ne írd felül rögtön a production példányt.

## 4. Local-auth DB

A local-auth DB külön SQLite security boundary. Password hash, TOTP secret és recovery hash található benne, ezért backupját secretként kezeld.

Stopped/maintenance baseline mellett:

```bash
sqlite3 /srv/velran/auth/users.db ".backup '/backup/local-auth.db'"
```

A backup legyen titkosított storage-on, szűk hozzáféréssel és dokumentált retentionnel. Restore után a file és parent directory permissionjeit újra ellenőrizni kell.

Local-auth DB elvesztése nem pótolható az application DB-ből.

## 5. AppFs data root

A teljes deklarált `data_root` állapotot mentsd, ne csak a jelenleg HTTP-n publikált media könyvtárakat.

Stopped baseline mellett például:

```bash
tar -C /srv/velran -cpf data-root.tar data
```

A restore-nál őrizd meg a szükséges ownership/permission értékeket, majd a service userrel ellenőrizd, hogy csak a deklarált root írható. Symlinkkel ne próbálj AppFs confinementet megkerülni.

## 6. Backup manifest

Minden recovery pointhoz legyen kis, secretmentes manifest. Példa:

```text
backup_id=2026-09-04T180000Z
release_id=2026-09-04.1
server_sha256=...
app_source_sha256=...
migrations_sha256=...
config_sha256=...
db_backend=postgresql
app_db_backup_sha256=...
local_auth_backup_sha256=...
data_root_backup_sha256=...
```

A starter `deploy/release-record.sh` read-only módon segít a release/source/migration/config hashok rögzítésében. DB backup vagy secret tartalmát nem olvassa.

## 7. Restore drill — nem productionon

A restore-próba mindig izolált környezetben induljon:

```text
új/üres restore DB
→ DB backup restore
→ local-auth restore, ha használatban van
→ AppFs restore külön rootba
→ exact immutable release kiválasztása
→ restore-test config külön host/porttal és secretekkel
→ velran-server --check-config
→ velran-cli check main.vrn
→ migrate verify
→ velran-server start izolált listeneren
→ live + ready
→ alkalmazás-specifikus smoke test
→ audit/log ellenőrzés
```

A starter `deploy/restore-verify.sh` a **nem mutáló** config/source/migration ellenőrzési részt automatizálja. Nem indít production listener-t és nem futtat `migrate apply`-t.

A restore drill eredményét dátummal, backup ID-val és release ID-val rögzítsd. A „backup elkészült” önmagában nem release gate; friss, sikeres restore-bizonyíték kell a szervezeti RPO/RTO szerint.

## 8. Upgrade contract

Ajánlott production upgrade:

```text
release artifact + hash kész
→ migration verify
→ kompatibilitási döntés
→ konzisztens backup + restore readiness
→ migrate apply
→ migrate verify
→ új immutable release switch
→ controlled restart
→ live + ready + smoke
→ logs/metrics/audit
```

### Expand/contract

Ha gyors application rollbacket akarsz megtartani, az új schema először legyen backward-compatible a régi alkalmazással:

1. **expand** — add új nullable/defaultolt oszlopot/táblát/indexet úgy, hogy a régi app tovább fusson;
2. deployold az új alkalmazást;
3. adat-backfill külön kontrollált műveletként, ha kell;
4. csak egy későbbi release-ben **contract** — régi oszlop/constraint eltávolítása, amikor rollback window már lezárult.

Destruktív rename/drop és azonnali NOT NULL/shape-váltás megszüntetheti az app-only rollback lehetőségét. Ezt release előtt explicit jelöld.

## 9. Rollback döntési fa

### A. App release hibás, schema backward-compatible

```text
forgalom kivétele
→ previous immutable `current`
→ controlled restart
→ live + ready + smoke
```

DB restore nem kell; a hiba után keletkezett üzleti adat megmarad.

### B. App release hibás, schema nem backward-compatible, de forward fix lehetséges

**Preferált:** forward fix új immutable release-szel vagy új forward migrationnel. Ne futtass vakon kézzel reverse SQL-t.

### C. Schema/data sérült, recovery szükséges

Ez már **restore**, nem egyszerű rollback:

```text
írások leállítása
→ választott recovery point rögzítése
→ DB + auth DB + AppFs konzisztens restore
→ hozzá illő release/config kiválasztása
→ verify + smoke izoláltan vagy maintenance környezetben
→ kontrollált forgalomba állítás
```

A recovery point utáni írások elveszhetnek. Ezt az operatornak az incidens döntésben explicit vállalnia és dokumentálnia kell.

## 10. Tiltott rövidítések

- élő SQLite DB egyszerű `cp` backupnak nevezése;
- csak application DB mentése, ha AppFs üzleti adatot tartalmaz;
- backup sikerének feltételezése restore-próba nélkül;
- automatikus `down migration` application rollback részeként;
- production DB-re restore-próba;
- migration credential átadása az application service-nek;
- secret value kiírása release/backup manifestbe;
- Redis session/cache restore kötelezővé tétele disaster recoveryhez.

## 11. Release exit criteria M46 után

- state inventory dokumentált;
- RPO/RTO szervezeti értéke rögzített;
- backup encryption/retention/access policy dokumentált;
- legutóbbi restore drill sikeres és azonosítható;
- migration kompatibilitás (`backward-compatible` vagy `requires forward-only`) release-ben rögzített;
- app-only rollback eljárás próbált;
- destructive recovery eljárás tulajdonosa és jóváhagyási útja ismert;
- Redis elvesztésének hatása elfogadott: session reset/cache rebuild/rate-limit window reset.
