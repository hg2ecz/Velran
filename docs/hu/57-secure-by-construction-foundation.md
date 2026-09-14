<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Secure-by-construction nyelvi alap

A Velran célja, hogy a gyakori webbiztonsági követelményekből nyelvi szemantika legyen, amikor azok megbízhatóan kikényszeríthetők. A biztonságos út legyen egyben a legegyszerűbb út.

## Explicit route-hozzáférés

Minden route-nak kötelező kimondania a hozzáférési policyt:

```velran
route home GET "/" public => home;
route account GET "/account" auth user => account;
route billing POST "/billing" auth mfa => billing;
route admin GET "/admin" auth role Admin => admin;
```

Ha nincs `public` vagy `auth ...`, a compiler `SEC-A01-001` hibával leáll. Ezzel megszűnik az implicit publikus végpont.

## Biztonságos redirect alapból

Actionből csak compiler által birtokolt, lokális útvonal-literal használható:

```velran
return Ok(redirect("/account"));
```

Dinamikus `String` (`redirect(target)`), protokoll-relatív (`//...`) vagy külső URL nem fordul le. A cél az open redirect hibák strukturális kizárása.

## Automatikusan korlátos külső String

A query/form/JSON `String` mezők explicit `length` szabály nélkül automatikusan `0..4096` hosszhatárt kapnak. A fejlesztőnek tehát nem kell minden mezőre ismétlődő boilerplate-et írnia, de szükség esetén szűkebb szabály adható:

```velran
route search GET "/search"
    query q<String>
    validate q length 1 120
    public => search;
```

## Stabil security diagnosztikák

- `SEC-A01-001`: hiányzó explicit route-hozzáférés
- `SEC-A01-002`: nem biztonságos redirect literal
- `SEC-A01-003`: dinamikus String redirect
- `SEC-A05-001`: nem biztonságos SQL
- `SEC-A05-002`: nem biztonságos HTML

Az OWASP-kódok kockázati családhoz való hozzárendelést jelentenek, nem teljes OWASP-megfelelőségi állítást.

## Tervezési sorrend

1. ami lehet, legyen reprezentálhatatlanul veszélyes;
2. ha ez nem lehetséges, legyen automatikus secure default;
3. ha döntés kell, legyen explicit capability/policy;
4. warning csak akkor, amikor a compiler nem tud elég biztosan fail-closed módon dönteni.

A következő rétegek: trust/taint típusok, `Secret<T>`/`Sensitive<T>`, authorization proof típusok, typed route targetek, SSRF-safe egress capabilityk, explicit API projection és bounded collection/input típusok.
