<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Velran — a security-first web application language and server with Rust syntax.

Velran egy Rust-alapú, webalkalmazás-fejlesztésre specializált nyelv/runtime/server ökoszisztéma. A V1 fókusza: secure-by-default működés, typed input/output, compiler által kikényszerített policyk, explicit capabilityk, auditálhatóság és production üzemeltethetőség.

## Secure-by-construction nyelvi alap

A route-ok többé nem publikusak implicit módon: minden route `public` vagy `auth ...` policyt kér. A dinamikus String redirect tiltott, a normál redirect csak compiler által ellenőrzött lokális útvonal-literal lehet. A külső query/form/JSON `String` értékek explicit szabály nélkül automatikus 4096 karakteres felső korlátot kapnak. Részletek: [`docs/hu/57-secure-by-construction-foundation.md`](docs/hu/57-secure-by-construction-foundation.md).


## Első lépések

Webfejlesztőként innen indulj:

1. [Webalkalmazás-fejlesztői kézikönyv](docs/hu/README.md)
2. [Gyors kezdés](docs/hu/01-gyors-kezdes.md)
3. [Starter project és production deployment](docs/hu/29-production-deployment.md)
4. [Security checklist](docs/hu/15-security-checklist.md)
5. [V1 kiadási jegyzet](RELEASE-NOTES-V1.0_hu.md)

A teljes referenciaalkalmazás: `examples/starter-project/`.

A csak verifikációra szolgáló elutasítási fixture-ök és manifestek a `tests/` alatt vannak; a felhasználóknak szánt példák akkor is az `examples/` alatt maradnak, ha a `verify.sh` is fordítja őket.

## Ellenőrzött fejlesztési mérföldkő

A jelenlegi Velran implementáció ebben az ellenőrzött baseline-ban konszolidálva van. A prioritási sorrend változatlan: **biztonságos webes működés → fejlesztői produktivitás → futási/fordítási teljesítmény**, mindezt clean-code felelősségi határokkal.

A jelenlegi forrásfa valódi Rust toolchainnel elérte a repository-szintű fejlesztési gate-et: a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás sikeresen lefutott. Ezt az állapotot a dokumentáció **Ellenőrzött fejlesztési mérföldkőként** rögzíti.

Ez nem helyettesíti a cél production környezet bizonyítékait. A deployment topológia, recovery-próba, secret/TLS/reverse-proxy konfiguráció, operátori elfogadás és a környezetfüggő performance/security evidence továbbra is release-feladat. Részletek: [`RELEASE-CHECKLIST.md`](RELEASE-CHECKLIST.md) és [`ITERATION-1-NOTES_hu.md`](ITERATION-1-NOTES_hu.md).

## Build és verifikáció

A workspace Rust edition 2024-et használ, de nincs konkrét compiler/toolchain verzióhoz pinelve. A reprodukálható dependency feloldást a committed `Cargo.lock` és a `--locked` build adja.

**Futtatási követelmény: minden Velran alkalmazást futtató gépen szükség van a `rustc` fordítóra.** A Velran egyetlen alkalmazás-végrehajtási backendje a natív Rust backend: az ellenőrzött Velran kód safe Rust kóddá alakul, majd a `rustc` immutable `cdylib`-bé fordítja az aktiválás előtt. Emiatt önmagában a `velran-server` bináris telepítése `rustc` nélkül nem elegendő. A `rustc` az aktív toolchainből automatikusan felismerhető, vagy a `server.toml` `[native]` szekciójában abszolút útvonallal explicit megadható. A Cargo magának a Velrannak a forrásból történő buildjéhez kell; az alkalmazások aktiválásakor a backend fordító közvetlenül a `rustc`.

```bash
./verify.sh
```

A release előtt a teljes `verify.sh` legyen zöld. A kiadási döntéshez a canonical angol [release checklist](RELEASE-CHECKLIST.md) szerinti automatizált és környezetfüggő evidence is szükséges.

## Production indítás

Productionban a TOML config az elsődleges interface:

```bash
velran-server --config /usr/local/etc/velran/server.toml
```

Preflight:

```bash
velran-server --config /usr/local/etc/velran/server.toml --check-config
```

Precedence:

```text
defaults < TOML config < célzott CLI override
```

A hosszú production CLI flaglista nem ajánlott; a stabil policyk trusted configban legyenek. SIGHUP újranyitja a logokat és behind-proxy módban tranzakciósan újratölti a domain/application hosting állapotot. Az alkalmazás-források változását a közös source-reload supervisor automatikusan is észleli; a process-szintű beállításokhoz továbbra is restart kell. Részletesen: `docs/hu/38-automatikus-forraskod-reload.md`.

## Debian csomag

Debian/Ubuntu build gépen a `make deb` telepíthető `velran_1.0.0-1_<arch>.deb` csomagot készít. A Debian csomagkezelő által birtokolt telepítés szándékosan `/usr/bin` és `/etc/velran` útvonalakat használ a kézi `/usr/local` telepítés helyett. Részletek: [docs/hu/36-debian-csomag.md](docs/hu/36-debian-csomag.md).

## V1 fő capabilityk

- typed routing/forms/JSON, domain és nominális típusok, exhaustive enum `match`;
- typed SQL, statikus DB row-bound, optimistic locking és explicit transaction outcome;
- local/LDAP auth, TOTP/MFA, permission, object/mutation authorization és auth abuse protection;
- platform-owned session + purpose-safe token hash/expiry/revoke/rotation lifecycle;
- `Secret<T>` / `Sensitive<T>` flow, explicit public projection és trusted `redact(...)`;
- critical-operation contract audit/transaction/idempotency garanciákkal;
- tenant authority és compiler-enforced tenant isolation;
- verified webhook replay protection és staged/verified file publish;
- effect/capability modell és named SSRF-hardened outbound integration;
- typed HTTP metadata, generált CSP/security headerek és strict production policy;
- purpose/lifecycle typed crypto keyek és AES-256-GCM authenticated encryption;
- request/DB/file/outbound resource cap, hard deadline és cumulative I/O budget;
- structured security event/redaction/burst alerting alap;
- supply-chain capability/provenance lock és release evidence.

## Tudatos V1 non-goalok

A következők nem részei a jelenlegi V1 magnak: full-text search, background jobs, email küldés, scheduler, revision history, workflow engine, soft delete, admin CRUD generation, package/registry terjesztés, általános/wildcard vagy item-szintű re-export szemantika, S3 media, image resize/thumbnail, HTTP/2/3, SSE/WebSocket, OTel SDK és private cache.

Az ordinary library-elemekhez most szűk Rust-szerű item-szintű `pub` visibility tartozik: struct, enum, pure `fn`, inherent metódus és struct-mező alapból privát, modulhatáron át pedig csak explicit `pub` API érhető el. Maga az `impl` blokk nem lehet `pub`; az egyes metódusok igen. A page/action/query authorityját továbbra is route/capability policy adja, nem a visibility. A `pub mod` támogatott nested visibility-határ. A szűk `pub use path as alias;` forma csak package-rootból és csak már publikus modul-namespace-re támogatott; wildcard, közvetlen item-, privát modul- és privát tranzitív dependency re-export továbbra is fail-closed. Compute kódban Rust-szerű `let mut` és közvetlen értékadás (`value = ...`, `array[index] = ...`) használható; a legacy `set` szintaxis tiltott. A tartós üzleti állapotmódosítás továbbra is explicit query/transaction/action útvonalakon történik.

## Operator dokumentáció

- [Server config](docs/hu/15-server-config.md)
- [Observability](docs/hu/09-observability.md)
- [Production deployment](docs/hu/29-production-deployment.md)
- [Backup/restore/upgrade/rollback](docs/hu/30-backup-restore-upgrade-rollback.md)
- [IPv6 egress](docs/32-ipv6-egress.md)
- [CLI workflow](docs/hu/33-cli-workflow.md)
- [`server.toml` referencia](docs/hu/34-server-toml-reference.md)
- [Production checklist](docs/hu/35-production-checklist.md)
- [Release checklist](RELEASE-CHECKLIST.md)

## Security

A canonical aktuális állapot: [SECURITY-STATUS.md](SECURITY-STATUS.md). A webes/production ellenőrzőlista: [docs/hu/15-security-checklist.md](docs/hu/15-security-checklist.md). A projekt fail-closed elve: ismeretlen vagy nem bizonyítható állapotnál ne legyen implicit permissive fallback.


Végrehajtás: egyetlen alkalmazás-végrehajtási út van: verifikált IR -> generált safe Rust -> közvetlen `rustc` -> immutable `cdylib` -> atomikus generation activation. Nincs VM vagy interpreter fallback; a natívan még nem támogatott konstrukciók fail-closed build hibát adnak, miközben az utolsó sikeres generation aktív marad.

### Natív lefedettség: nominal request inputok

A verifikált rustc backend külön típusként őrzi meg az `Email`, `Url`, `Slug` és a támogatott korlátos nominális domain request inputokat az EIR-ben és a cache identityben is. A nem támogatott constraint nem kerül fallback engine-re: a candidate build fail-closed módon elutasításra kerül. Az aktuális lefedettség: [`tests/NATIVE_STATUS.md`](tests/NATIVE_STATUS.md).


### Natív rustc backend konfiguráció

Production környezetben a natív backend explicit módon is konfigurálható:

```toml
[native]
enabled = true
cache_dir = "/var/cache/velran/native"
optimization = "release"
```

A `cache_dir` abszolút útvonal. A `rustc` opcionális; rustup-managed toolchain esetén az automatikus toolchain-felismerés az ajánlott. Ha explicit `rustc` útvonalat adsz meg, annak abszolút compiler-útvonalnak kell lennie. A teljes szekció elhagyásakor az automatikus rustc/cache felismerés működik.


## Natív példa-státusz

A native-only futtatható/pending példamátrix: [`tests/NATIVE_STATUS.md`](tests/NATIVE_STATUS.md).


## Történeti fejlesztési jegyzetek

A régi iterációs, benchmark- és build-fix jegyzetek a [`history/development-notes/`](history/development-notes/) alatt vannak elkülönítve. Ezek nem canonical dokumentumok, és tartalmazhatnak már megszüntetett szintaxist vagy köztes architektúrát.

### Rustc-first diagnosztika és biztonsági kapu

A natív backend a megbízható `rustc`-t használja Rust szintaxis- és típushatóságként, nem épít mellé második Rust fordítót. A Velran előtte csak a webes sandboxhoz szükséges ellenőrzéseket végzi el (capabilityk, unsafe/ambient authority tiltása, erőforrás- és security contractok), majd az izolált `rustc` fordítja a generált cdylibet. A natív fordítás hibája megőrzi a valódi, méretkorlátos rustc diagnosztikát konzolhoz/errorloghoz; nem-production `--debug-compile-errors` esetén source reload közben HTML-escaped fejlesztői hibaoldal is megjelenhet, miközben az utolsó érvényes generáció tovább szolgál.

### Rust-first inherent metódusok

A Velran most egy szándékosan szűk, Rust-szerű inherent-metódus felületet fogad el: `impl Type { fn method(&self, ...) { ... } }`, valamint authority-mentes associated functionöket, például `Type::new(...)`. Inherent implen belül a `Self` használható biztonságos paraméter- és visszatérési típuspozícióban, illetve struct literalban; a compiler verified lowering előtt a deklaráló structra oldja fel. A compiler nem másolja nyersen ezeket a törzseket a generált Rustba: minden metódust regisztrál, majd a meglévő verified pure-compute útvonalon fordít le, így megmarad a fuel/allocation elszámolás és a capability-határ. A publikus metódusok típusfelületét külön ellenőrzi, így privát struct nem szivároghat ki publikus paraméteren vagy visszatérési típuson keresztül. Az owned vagy mutable `self`, a generikus impl és a trait impl továbbra is fail-closed a későbbi iterációkig. Cross-package publikus structok, konstruktorok és immutable metódusok ugyanazokat a visibility- és package-határokat használják. A keletkező safe Rust típusellenőrzését és optimalizálását továbbra is a valódi `rustc` végzi.


### Modulok és importok

A Velran támogatja a modulhoz relatív `mod`, a nested API-határhoz használható `pub mod`, valamint az explicit `use path as alias;` namespace-alias formát. A `pub use path as alias;` szűk formája csak package-rootból és csak már publikus modul-namespace-re támogatott; wildcard, item-, privát modul- és privát tranzitív dependency re-export továbbra is fail-closed. A webes authorityt nem a `pub`, hanem a route/capability policy szabályozza. Lásd: `examples/module-imports/`.


## Licenc

A Velran a Mozilla Public License 2.0 (`MPL-2.0`) alatt érhető el. Copyright (c) 2026 Zsolt Krüpl. Részletek: [`LICENSE`](LICENSE) és [`COPYRIGHT`](COPYRIGHT). A harmadik féltől származó függőségekre a saját licenceik vonatkoznak.
