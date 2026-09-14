<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Database migration workflow

A schema migráció **nem** az alkalmazásszerver indulásának mellékhatása. Productionban külön CLI-művelet, külön credential és külön deploy lépés.

## Migration fájlok

```text
migrations/
  0001_create_products.sql
  0002_products_name_index.sql
```

Névforma:

```text
NNNN_name.sql
```

A verzió pozitív egész és egyedi. Már alkalmazott migráció fájlnevét, nevét vagy tartalmát ne módosítsd; a CLI SHA-256 checksumot tárol és eltérésnél megáll.

## Credential

Migrationhöz külön, magasabb jogosultságú DB credential legyen:

```bash
printf '%s\n' 'postgres://migration_user:...@db.internal/app?sslmode=verify-full' \
  > /run/secrets/migration-db-url
chmod 600 /run/secrets/migration-db-url
```

A CLI migration parancs szándékosan csak `--db-url-file` formát fogad el.

## Status

```bash
cargo run -p velran-cli -- migrate status \
  --dir migrations \
  --db-url-file /run/secrets/migration-db-url
```

Példa:

```text
0001 applied  create_products
0002 pending  products_name_index
```

A `status` nem hoz létre state table-t és nem ír az adatbázisba.

## Verify

```bash
cargo run -p velran-cli -- migrate verify \
  --dir migrations \
  --db-url-file /run/secrets/migration-db-url
```

Ellenőrzi többek között:

- applied migration helyben is megvan;
- név nem változott;
- SHA-256 checksum nem változott;
- nincs utólag beszúrt régebbi pending migráció egy már alkalmazott magasabb verzió alá.

## Apply

```bash
cargo run -p velran-cli -- migrate apply \
  --dir migrations \
  --db-url-file /run/secrets/migration-db-url \
  --lock-timeout-secs 30
```

A CLI adatbázis-szintű migration lockot kér. Egyszerre csak egy migrációs futás dolgozhat ugyanazon adatbázison.

## Backend viselkedés

### PostgreSQL

- advisory migration lock;
- migrationonként tranzakció;
- PostgreSQL dollar-quoted function body támogatott a statement splitterben.

### SQLite

- `BEGIN IMMEDIATE` writer lock a teljes futásra;
- a teljes batch egy tranzakcióban marad;
- hiba esetén rollback.

### MariaDB

- `GET_LOCK` / `RELEASE_LOCK`;
- a DDL nem garantáltan tranzakcionális;
- a migration state csak akkor íródik be, ha a fájl minden statementje sikeres;
- részleges DDL hiba esetén operator repair szükséges lehet.

## SQL fájl szabályok

- `.sql` fájl maximum 4 MiB;
- legfeljebb 10 000 migration file;
- symlinkelt migration file tiltott;
- migration directory maga sem lehet symlink;
- MySQL/MariaDB `DELIMITER` kliensdirektíva nem része az SQL protokollnak és nincs támogatva; a fájl tényleges SQL statementeket tartalmazzon.

## Production sorrend

Ajánlott deployment:

```text
backup / restore-readiness
→ migration verify
→ migration apply
→ migration verify
→ új alkalmazásverzió rollout
→ readiness ellenőrzés
```

Az alkalmazás runtime credentialje továbbra is least-privilege DML credential maradjon. A migration credentialet az alkalmazásszerver ne kapja meg.
