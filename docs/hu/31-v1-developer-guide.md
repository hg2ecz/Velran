<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 31. V1 fejlesztői átadási ellenőrzőlista

Ez a fejezet nem új feature-lista. Azt ellenőrzi, hogy egy webfejlesztő a V1 surface-t egyetlen konzisztens úton tudja-e használni és productionre átadni.

## 1. Projekt

- `main.vrn` az entrypoint;
- a forrás `mod`-okkal szervezett;
- a domain API-k `object` alatt csoportosítottak;
- route policy top-level marad;
- nincs textual include, runtime source read vagy implicit capability.

`mod` application-root relatív namespace és source-graph deklaráció: betölti a modult, de nem emeli annak neveit globális scope-ba. Modulhatáron túl `foo::Name` alakú hivatkozás kell. Az ordinary library-réteg item-szintű `pub` visibilityt támogat (struct/enum/pure fn/inherent method/struct field); az elemek alapból privátak. A `pub mod` és re-export szemantika még nincs bekapcsolva.

## 2. Input és domain

- route/form/JSON input typed;
- zárt választási halmazhoz `enum`;
- emailhez `Email`, URL-hez `Url`, SEO pathhoz `Slug`;
- cross-field equalityhez `same`;
- statikus String mintához bounded `pattern`;
- unique invariant authoritative helye a DB `UNIQUE` constraint, nem race-es preflight SELECT.

Általános `mut` local/self state nincs a V1-ben. Üzleti módosítás explicit query/transaction/action útvonalon történik.

## 3. Adatbázis

- SQL text compiler-owned;
- user érték kizárólag typed bind;
- mutation transactionben;
- konkurens szerkesztéshez `Changed`/optimistic locking;
- üzleti állapotváltozás és audit ugyanabban a transactionben, ha audit szükséges;
- `_velran_` namespace runtime-owned.

## 4. Web UX

- invalid form: 422 + field errors;
- sikeres POST: PRG, 303 redirect;
- egyszer használatos flash csak statikus compiler-owned üzenettel;
- stale/unique conflict: 409;
- canonical slug eltérés: same-route permanent redirect;
- user/session dependent oldal ne legyen public cache.

## 5. Security

Release előtt járd végig a [security checklistet](15-security-checklist.md). Kiemelten:

- HTTPS/public host/trusted proxy config;
- CSRF + Origin/Fetch Metadata;
- auth/authz route policy;
- AppFs és outbound network named policy;
- secret csak `*_file` formában;
- operator-controlled resource limits;
- server/access/audit napló szétválasztása.

## 6. Production átadás

Az ajánlott út:

```text
./verify.sh
→ immutable build/release hash
→ velran-server --config ... --check-config
→ migration status/verify
→ approved migrate apply
→ controlled restart
→ liveness/readiness
→ log/metric/audit review
→ backup/restore readiness
```

A server nem migrál automatikusan startupkor.

## 7. Recovery

- application DB + local-auth DB + AppFs data együtt recovery state;
- Redis nem üzleti source of truth;
- backup csak restore-drill után tekinthető bizonyítottnak;
- app rollback csak backward-compatible schema mellett egyszerű;
- automatikus down migration nincs.

## 8. Amit ne építs V1-kódból házilag

Ne készíts alkalmazásszinten saját alternatívát ezek helyett:

- raw SQL interpolation;
- raw HTML escape hatch;
- saját session/auth cookie;
- saját path traversal ellenőrzés AppFs helyett;
- saját SSRF allowlist named network policy helyett;
- race-es unique validator;
- numeric resource-limit emelés `.vrn` kódból;
- tetszőleges permanent redirect cél.

Ha a szükséges capability nincs a V1 surface-ben, azt új nyelvi/runtime feature-ként kell megtervezni, nem kerülőúttal megkerülni.
