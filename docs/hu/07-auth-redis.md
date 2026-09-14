<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 7. Authentication, session, Redis, LDAP és local user store

Velran két elsődleges login backendet támogat: **LDAP** vagy **local auth SQLite**. Egy server instance-ban csak az egyik legyen aktív.

## Session

Development/test: in-memory. Production/horizontális sessionhez Redis:

```bash
--redis-url-file /run/secrets/redis-url
```

Redis a shared session store, TOTP replay store és login-rate-limit backend.

## Local auth

Local/single-node alkalmazáshoz először külön auth DB-t hozz létre. A DB **nem az application DB**, ezért `.vrn` query nem fér hozzá credential adathoz.

Secret file:

```text
/run/secrets/local-auth-db-url:
sqlite:///srv/velran/auth/users.db?mode=rwc
```

Inicializálás:

```bash
cargo run -p velran-cli -- auth init \
  --db-url-file /run/secrets/local-auth-db-url
```

User létrehozás. Jelszó kizárólag file-ból:

```bash
printf '%s\n' 'a-very-long-random-password' > /run/secrets/new-user-password
chmod 600 /run/secrets/new-user-password

cargo run -p velran-cli -- auth user-add \
  --db-url-file /run/secrets/local-auth-db-url \
  --username alice \
  --password-file /run/secrets/new-user-password \
  --role User \
  --role Editor
```

Server:

```bash
velran-server \
  --app /srv/myapp/main.vrn \
  --local-auth-db-url-file /run/secrets/local-auth-db-url \
  --redis-url-file /run/secrets/redis-url \
  ...
```

### Password és account lifecycle

```bash
velran-cli auth password-set --db-url-file ... --username alice --password-file ...
velran-cli auth disable      --db-url-file ... --username alice
velran-cli auth enable       --db-url-file ... --username alice
velran-cli auth roles-set    --db-url-file ... --username alice --role User --role Publisher
```

A disable nem törli a usert és kívülről ugyanúgy invalid credentialnek látszik. Password-, role-, disable/enable- és TOTP-változás növeli az `auth_generation` értéket, ezért a korábbi session a következő requestnél visszavonódik.

## TOTP és recovery

```bash
cargo run -p velran-cli -- auth totp-enroll \
  --db-url-file /run/secrets/local-auth-db-url \
  --username alice
```

A CLI kiírja a TOTP secretet hex/base32 formában, egy `otpauth://` URI-t és a recovery kódokat. A recovery kódokat a DB csak hashként tárolja; egy sikeres használat törli a kódot.

Login form `totp` mezőjébe 6 jegyű TOTP vagy egy recovery code írható.

```bash
velran-cli auth totp-disable --db-url-file ... --username alice
```

`--require-totp` esetén olyan user sem léphet be, akinek nincs TOTP enrollmentje.

## LDAP

Vállalati/shared identity esetén továbbra is LDAPS használható. Local auth és LDAP ugyanabban a processben nem kombinálható; ez elkerüli a névütközés és backend-prioritás bizonytalanságát.

## Authorization

```text
route account GET "/account" auth user => account;
route admin GET "/admin" auth role Admin => admin;
route security GET "/security" auth mfa => security;
```

A local auth role a dedikált auth DB-ből, LDAP role pedig trusted role mappingből jön. Kliens által beküldött role soha nem authority.

## Jelenlegi korlát

M25 local auth SQLite single-node identity store. Több application instance esetén LDAP a jelenlegi shared identity megoldás; shared SQL local-auth backend későbbi bővítés lehet.

Nincs még publikus registration, forgot-password email vagy self-service TOTP enrollment endpoint. Ezekhez külön token/email/rate-limit security workflow kell.
