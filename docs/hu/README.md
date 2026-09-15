<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Velran webalkalmazás-fejlesztői kézikönyv

**A projekt hivatalos teljes neve:** Velran — a security-first web application language and server with Rust syntax.

**Rövid leírásként a „Rust Web” szókapcsolat használható, de a projekt hivatalos neve a fenti teljes név.** Ez a kézikönyv webfejlesztőnek szól: hogyan készíts, tesztelj és adj át productionre egy valódi Velran alkalmazást. A fő útvonal egy ajánlott megoldást mutat; a compiler/runtime belső részletei csak ott jelennek meg, ahol fejlesztői vagy security döntéshez szükségesek.

## V1 ajánlott tanulási út

### 1. Első működő alkalmazás

1. [Gyors kezdés](01-gyors-kezdes.md)
2. [Projektstruktúra, modulok és Slug](21-modules-slugs-project-layout.md)
3. [Nyelvi és HTML alapok](03-nyelv-html.md)
   - [Numerikus operátorok, matematika és monoton időmérés](43-matematika-es-idomeres.md)
   - [String műveletek](45-string-muveletek.md)
   - [Statement terminátorok](55-statement-terminatorok.md)
   - [Verifikált pure függvények](58-verifikalt-pure-fuggvenyek.md)
4. [Routing, input és validáció](04-routing-input.md)
5. [Adatbázis és typed SQL](05-adatbazis.md)
6. [CRUD és tranzakciók](06-crud.md)
7. [Domain objectek](22-domain-objects.md)

### 2. Üzleti webalkalmazás

8. [Date, DateTime, Uuid és Decimal](11-business-types.md)
9. [Enumok](23-enums.md)
10. [Domain-validáció](26-domain-validation.md)
11. [Újrafelhasználható formok](12-forms.md)
12. [PRG, flash és conflict UX](28-prg-flash-conflict.md)
13. [Optimistic locking](../24-optimistic-locking.md)
14. [Üzleti audit trail](25-business-audit-trail.md)
15. [Canonical URL és régi slugok](27-canonical-url.md)

### 3. Auth, API és tartalom

16. [Auth és Redis](07-auth-redis.md)
17. [Objektumszintű authorization](20-object-authorization.md)
18. [JSON API és CORS](05-json-api-cors.md)
19. [Komponensek és layoutok](14-components-layouts.md)
20. [Biztonságos Markdown](16-markdown-rich-text.md)
21. [Képek és media library](17-media-library.md)
22. [Upload és AppFs](09-upload-appfs.md)
23. [Public cache](13-cache.md)
24. [Statikus assetek](06-static-assets.md)

### 4. Production

25. [Database migrations](10-database-migrations.md)
26. [Rate limiting](08-rate-limiting.md)
27. [Resource profile-ok](10-resource-profile.md)
28. [HTTPS és böngészőbiztonság](08-https-security.md)
29. [Server konfiguráció](15-server-config.md)
30. [Lifecycle és health](07-lifecycle-health.md)
31. [Observability](09-observability.md)
32. [Dependency security és reprodukálható build](../19-dependency-security.md)
33. [Tesztelés és hibakeresés](12-teszteles-hibak.md)
34. [Web security checklist](15-security-checklist.md)
35. [Production deployment és starter project](29-production-deployment.md)
36. [Backup, restore, upgrade és rollback](30-backup-restore-upgrade-rollback.md)
37. [V1 fejlesztői átadási ellenőrzőlista](31-v1-developer-guide.md)
38. [CLI és napi workflow](33-cli-workflow.md)
39. [`server.toml` konfigurációs referencia](34-server-toml-reference.md)
40. [Production checklist és minták](35-production-checklist.md)
41. [Debian csomag készítése és dpkg telepítés](36-debian-csomag.md)
42. [Több domain kiszolgálása](37-tobb-domain-kiszolgalas.md)
43. [Automatikus alkalmazás-forráskód reload](38-automatikus-forraskod-reload.md)

## V1 modulnévtér-szabály

Az R48 óta a `mod` source-graph deklaráció és namespace-határ, nem globális include. A `mod foo;` pontosan a `<app-root>/foo.vrn` forrást tölti be `foo` névtérként; más modulból `foo::Name` alakú kvalifikált hivatkozás szükséges. A nested `foo::bar` modul kizárólag `foo/bar.vrn`, nincs `mod.vrn`, `../`, `self::`, `super::`, `crate::` vagy automatikus almappa-discovery. A részletes szerződés: [Projektstruktúra, modulok és Slug](21-modules-slugs-project-layout.md).

## V1 aktuális kifejezés- és string surface

Az aritmetikai mag ellenőrzött `+`, `-`, `*`, `/`, `%` műveleteket, `i64` shiftet (`<<`, `>>`), bitenkénti `&`, `^`, `|` operátorokat és `bool` logikát (`!`, `&&`, `||`) ad; a `&&` és `||` short-circuit. Az f32 builtin készlet része többek között az `ln`, `log10`, `log`, `exp`, `pow`, `round`, `floor` és `ceil`. Részletesen: [Numerikus operátorok, f32 matematika és monoton időmérés](43-matematika-es-idomeres.md).

A Unicode-tudatos String API Rust-szerű metódusokat használ: `.trim()`, `.trim_start()`, `.trim_end()`, `.to_lowercase()`, `.to_uppercase()`, `.chars().count()`, `.contains()`, `.starts_with()`, `.ends_with()`, `.replace()` és `.repeat()`. Ahol a webes biztonsági szerződés explicit elemszám-korlátot igényel, ott framework helper marad, például `splitBounded(...)`. Részletesen: [String műveletek](45-string-muveletek.md) és [Reguláris kifejezések](48-regexp.md).

Az egyszerű statementek és a blokk nélküli `mod`/`route` deklarációk explicit `;` terminátort használnak. A sortörés nem terminátor. Részletesen: [Statement terminátorok](55-statement-terminatorok.md).

## V1 nyelvi határ: `pub` és mutáció

A jelenlegi surface szűk, Rust-szerű item-szintű `pub` visibilityt ad az ordinary library-réteghez; az elemek alapból privátak. A `mod` továbbra is namespace/source-graph mechanizmus, a `pub mod` mellett pedig csak package-root szintű, explicit `pub use path as alias;` modul-re-export engedélyezett; wildcard, közvetlen item-, privát modul- és privát tranzitív dependency re-export fail-closed módon tiltott.

Compute kódban viszont a Rust-szerű mutáció támogatott: `let mut`, közvetlen assignment és a támogatott compound assignment alakok használhatók. A legacy `set` szintaxis tiltott. Üzleti/tartós állapotmódosítás továbbra is action/query/transaction/capability útvonalon történik.

## Státuszjelölések

- **IMPLEMENTÁLT** — tényleges kódút van a workspace-ben.
- **TRUSTED RUST ADAPTER** — runtime/integrációs Rust felület, de még nem `.vrn` standard API.
- **NEM TÁMOGATOTT** — tudatos non-goal vagy későbbi feature.

- [IPv6-ready outbound egress](../32-ipv6-egress.md)
- [CLI és napi workflow](33-cli-workflow.md)
- [`server.toml` konfigurációs referencia](34-server-toml-reference.md)
- [Production checklist és minták](35-production-checklist.md)
- [Debian csomag készítése és dpkg telepítés](36-debian-csomag.md)

## LaTeX könyv

A fejezetenként `\input`-olt, webalkalmazás-fejlesztőknek szóló könyv forrása: [`../book/hu/README.md`](../book/hu/README.md). Az angol kiadás canonical; a magyar kiadás ugyanazokat a technikai szerződéseket követi.

## Aktuális security architektúra

A canonical állapotösszefoglaló: [../../SECURITY-STATUS.md](../../SECURITY-STATUS.md). A részletes, témánkénti security dokumentáció az angol [`../README.md`](../README.md) indexből érhető el; a magyar kézikönyv a fejlesztői/operátori útvonalra koncentrál.
