<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Tenant membership authority

A Velran platform-auth session most már trusted, korlátos tenant-membership claim halmazt hordoz. Ez lesz a későbbi, compiler által kikényszerített `scoped by tenantId` modellrendszer valódi authority alapja.

## Biztonsági tulajdonságok

- A tenant ID kanonikus ASCII azonosító, legfeljebb 128 byte.
- Egy session legfeljebb 64 egyedi membershipet hordozhat.
- Anonymous sessionnek nincs membership claimje.
- Local auth esetén a membership az auth adatbázisból jön, nem alkalmazás-inputból.
- Membership módosítás növeli az `auth_generation` értékét, ezért a régi session érvénytelenné válik.
- LDAP módban `auth.memberships_file` / `--auth-memberships-file` használható `username=tenant,tenant` formában.
- LDAP esetén a role + membership mapping determinisztikus hash-e kerül a session generation mezőbe; mapping változáskor a régi, akár Redisben tárolt session a következő requestnél fail-closed invalidálódik.
- A runtime csak belső `__authMemberships` értékként kapja meg. A nyelv még nem tesz ki ambient `currentTenant` változót.

## Local auth adminisztráció

```text
velran-cli auth user-add ... --tenant acme --tenant beta
velran-cli auth memberships-set --db-url-file auth-url.txt --username alice --tenant acme
```

A membership változás authentication-authority változás, ezért a korábbi authenticated sessionök generációeltérés miatt érvénytelenek lesznek.

## LDAP/operator mapping

```toml
[auth]
memberships_file = "memberships.txt"
```

```text
alice=acme,beta
bob=acme
```

Hibás tenant ID, duplikált user vagy duplikált membership startup hibát okoz.

## Miért nincs még `currentTenant`?

A membership és az aktív tenant nem ugyanaz. A Velran szándékosan nem választja ki automatikusan az első membershipet aktív tenantként. A következő tenant-isolation iteráció a route/modell tenant scope-ot ezekhez a trusted claim-ekhez köti, és compiler proofot termel.
