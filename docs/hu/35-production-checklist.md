<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Production checklist és minták

A könyv függeléke összefoglalja a production telepítéshez használható gyorsreferenciát. A részletes indoklásért lásd a security, observability, deployment, recovery és server-config dokumentációt.

## Alap útvonalak

- binárisok: `/usr/local/bin/velran-server`, `/usr/local/bin/velran-cli`
- konfiguráció: `/usr/local/etc/velran/server.toml`
- secret fájlok: `/run/secrets/velran/`
- alkalmazás/release/data: `/srv/velran/`
- logok: `/var/log/velran/`

## Release gate

1. locked release build és `velran-cli check`;
2. migration `status -> verify -> apply -> verify`;
3. `velran-server --check-config`;
4. secret-, proxy-, auth-, CSRF/CORS-, DB-, storage-, Redis- és egress-policy review;
5. systemd/cgroup/resource limitek;
6. log/metrics/readiness;
7. friss és restore-drillel bizonyított backup;
8. rollback terv;
9. deploy után liveness + readiness + üzleti smoke.

Konkrét minták:

- `examples/systemd/velran.service`
- `examples/logrotate/velran`
- `examples/starter-project/deploy/`
