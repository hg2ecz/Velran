<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 33. Velran CLI és napi workflow

A publikus binárisok:

```text
/usr/local/bin/velran-cli
/usr/local/bin/velran-server
```

## Feladatok

- `velran-cli check <app.vrn>`: compiler-szintű source ellenőrzés.
- `velran-cli migrate status|verify|apply --dir <migrations> --db-url-file <path>`: explicit migration lifecycle.
- `velran-cli auth ... --db-url-file <path>`: local-auth identity store adminisztráció.
- `velran-server --config /usr/local/etc/velran/server.toml --check-config`: startup preflight.
- `velran-server --config ... --print-effective-config`: secret-redacted effective config.

A CLI nem HTTP server és nem olvassa általánosan a teljes `server.toml`-t. Migration és local-auth admin esetén a szükséges DB URL külön secret fájlból érkezik.

## Fejlesztői ciklus

```text
edit
-> velran-cli check
-> migrate verify
-> szükség esetén dev migrate apply + verify
-> velran-server --check-config
-> server start
-> HTTP/integration test
```

## Release ciklus

```text
immutable release
-> velran-cli check
-> velran-server --check-config
-> migrate status + verify
-> backup/recovery gate
-> approved migrate apply
-> migrate verify
-> controlled restart/switch
-> live + ready + smoke
-> log/metric/audit review
```

A server startup nem futtat automatikus migrationt.

## Local-auth parancsok

```text
velran-cli auth init
velran-cli auth user-add
velran-cli auth password-set
velran-cli auth disable
velran-cli auth enable
velran-cli auth roles-set
velran-cli auth totp-enroll
velran-cli auth totp-disable
```

A TOTP enrolment secretet és egyszer megjelenő recovery code-okat ír stdout-ra; ezt csak kontrollált operátori terminálon futtasd.

## Secret file contract

A DB URL secret file regular, nem symlink, legfeljebb 16 KiB és pontosan egy nem üres sor. A password file regular, nem symlink, legfeljebb 4096 byte és egyetlen sor; a jelszó 12..1024 byte.

## Platform build

Ha magát a Velran workspace-et release-eljük:

```bash
./verify.sh
cargo build --locked --release -p velran-server -p velran-cli
```

Eredmény:

```text
target/release/velran-server
target/release/velran-cli
```
