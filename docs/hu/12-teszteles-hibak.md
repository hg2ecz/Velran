<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 12. Tesztelés és hibakeresés

Minden változás után:

```bash
./verify.sh
```

Release minimum:

```bash
cargo check --workspace
cargo test --workspace
```

## Fontos integration tesztek

- SQLite CRUD;
- PostgreSQL és MariaDB valós backend;
- Redis session/TOTP replay;
- TLS cert/hostname;
- AppFs `openat2` escape/symlink/xdev;
- multipart size/cleanup;
- cgroup memória/CPU/PID;
- slowloris/request-smuggling;
- load smoke;
- health/readiness dependency failure;
- SIGTERM graceful drain és grace-timeout.

## Tipikus HTTP hibák

| Helyzet | HTTP |
|---|---:|
| hibás input/request | 400 |
| nem található | 404 |
| rossz method | 405 |
| Host mismatch | 421 |
| túl nagy upload/body | 413 |
| rossz content type | 415 |
| túl nagy header | 431 |
| HTTPS szükséges | 426 |
| timeout/resource/DB availability | 503 |
| belső hiba | 500 |

A kliens ne kapjon SQL-t, secretet, fizikai filesystem pathot, session tokent vagy stack trace-et.

## JSON/CORS M19

A `verify.sh` ellenőrzi az `examples/json-api/app.vrn` fordítását és a kevert body-mode negatív fixture-öket. A server unit tesztek külön fedik:

- duplikált JSON kulcs;
- nested/float JSON input tiltását;
- `Accept` negotiationt;
- CORS preflight Origin + route scope-ot;
- credentialed cross-origin state-change policyt.
