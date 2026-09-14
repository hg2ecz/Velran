<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 15. Web security checklist

Ez a lista nem helyettesíti a Velran compiler/runtime garanciáit. A célja az, hogy production átadáskor azokat a döntéseket ellenőrizd, amelyek szükségképpen alkalmazás- vagy üzemeltetői kontextust igényelnek.

A nyelv aktuális security státusza: [`../../SECURITY-STATUS.md`](../../SECURITY-STATUS.md).

## Amit normál Velran kódban már ne kézzel oldj meg

A platform/compiler kezeli vagy kikényszeríti többek között:

- explicit route access policy;
- typed SQL és strukturált HTML boundary;
- typed lokális route redirect;
- bounded/validált request input;
- Secret/Sensitive adatfolyam és explicit public projection;
- object- és mutation-authorization proof;
- permission + MFA + critical-operation contract;
- platform-owned session, token rotation/revocation és auth-generation invalidáció;
- tenant isolation;
- idempotens critical operation;
- verified webhook + replay protection;
- staged/verified file upload;
- named outbound integration / SSRF policy;
- production security contract;
- CSP/security headerek és typed HTTP metadata;
- key-purpose/lifecycle és AEAD user-data encryption;
- exhaustive sum type match + `CommitUnknown` transaction outcome;
- DB/request/outbound/file resource limitek;
- auth abuse protection;
- strukturált security event, redaction és burst alerting alap.

Ha ezekhez az alkalmazásban külön security plumbingot kezdesz írni, előbb ellenőrizd, hogy nem a meglévő nyelvi/platform capabilityt kerülöd-e meg.

## Fejlesztői ellenőrzések

- A route authority a valódi domain-intentet fejezze ki (`public`, auth/permission/MFA/critical/webhook/tenant), ne gyengébb policy mögé tedd a handlert.
- Tenant ID, role, price, ownership vagy permission authority ne kliens bodyból származzon.
- Sensitive adat csak szükséges authorization proof után kerüljön outputba; Secret ne kerüljön generic output/audit sinkbe.
- Kritikus side effectnél használd a critical contractot és idempotencyt; `CommitUnknown` ágat ne kezeld automatikusan retryként.
- Külső HTTP célhoz csak named integrationt használj; ne építs user-inputból hostot/URL-t.
- Uploadnál a kliens filename/MIME csak untrusted metadata; public image csak verified publish transitionből jöjjön.
- DB list query legyen statikusan bounded/paginált.
- Ne kérj nagyobb route/resource profilt pusztán azért, hogy egy unbounded algoritmus átmenjen.
- Sensitive/Secret logolás helyett szükség esetén explicit `redact(...)` értéket használj.

## Production átadás előtt

- `production { https required; debug disabled; hsts required; database tls required; }` policy legyen összhangban a tényleges topologyval.
- Reverse proxy csak explicit trusted proxy CIDR-rel legyen authority; forwarding metadata máshonnan fail-closed.
- Redis-backed session/abuse/idempotency/replay state rendelkezésre állása és fail-closed viselkedése legyen elfogadott.
- DB credential legyen least privilege; migration credential ne legyen az alkalmazás service-é.
- AppFs/data/upload/secret root jogosultságok és confinement legyenek ellenőrizve.
- Named egress target host/port/CIDR/TLS policy legyen review-zva.
- `Cargo.lock`, dependency capability policy és `VELRAN-SUPPLY-CHAIN.lock` legyen konzisztens.
- Security event/log pipeline ne tartalmazzon cookie-t, tokent, credentialt, raw principal/IP mezőt vagy teljes request bodyt.
- Burst alert thresholdok és downstream log/alert routing legyen üzemeltetőileg értelmezhető.
- Backup/restore és rollback gyakorlat legyen dokumentálva és tesztelve.

## Kötelező release gate

```bash
cargo fmt --all -- --check
cargo metadata --locked --format-version 1 >/dev/null
cargo check --workspace --locked
cargo test --workspace --locked
./verify.sh
```

A shell architecture/clean-code gate önmagában nem jelent build-green állapotot.

## Fontos fail-closed evidence

- invalid forwarding / Host / Origin / CSRF / authorization / webhook / replay / idempotency / resource-limit állapot ne váljon permissive fallbackká;
- authentication limiter store hibája ne kapcsolja ki csendben az abuse protectiont;
- invalid response metadata ne kerüljön wire-ra;
- stale Cargo/supply-chain lock ne legyen release-elhető;
- unknown transaction outcome ne legyen implicit retry-safe;
- security eventbe ne kerülhessen arbitrary request payload vagy secret.
