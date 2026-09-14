<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Route budget profilok

A Velran minden kérésre automatikusan alkalmazza a platform request budgetjét. Az alkalmazáskódnak nem kell külön bekapcsolnia az instruction- és allocation-limitet.

A szándékosan drágább, vagy külön concurrency/runtime profilt igénylő route operátor által definiált névvel ellátott profilt kérhet:

```vrn
#[page]
fn report(ctx: PageContext) -> Result<Html, PageError> {
    let total = 40 + 2;
    return Ok(html {<p>{{ total }}</p>});
}

route report GET "/report"
    auth user
    budget report
    => report;
```

A `budget report` nem ad numerikus limiteket az alkalmazás forrásában. A trusted szerver resource-profile konfiguráció birtokolja a `max_instructions`, `max_allocated_bytes` és `max_concurrent` értékeket. Így az alkalmazáskód nem tudja észrevétlenül gyengíteni a production ceilingeket.

## Biztonsági tulajdonságok

- Minden request kap platform default budgetet akkor is, ha nincs `budget` klauzula.
- A named route budget továbbra is a request hard ceilingen belül marad.
- Ismeretlen profil a szerver config/preflight során fail-closed hibát ad, mert a fordított program rögzíti a route budget használatát.
- A `budget default` tiltott (`SEC-A10-002`); a default implicit, nem kell hozzá boilerplate.
- Route-szintű budget és handler-szintű `with resource ...` együtt tiltott (`SEC-A10-001`). Így egy handlernek egyértelmű resource authorityja van, és egy belső profil nem kerülheti meg véletlenül a route célzott korlátját.
- A named profilok meglévő concurrency semaphore-ja is érvényes, ezért a drága route-ok izolálhatók a normál forgalomtól.

## Fejlesztői élmény

A normál route rövid marad:

```vrn
route home GET "/" public => home;
```

Csak a kivételes route kér profilt:

```vrn
route export GET "/export" auth user budget export => export;
```

A fejlesztő intentet mond (`export`), a konkrét limitek pedig production operátori policyk maradnak.

## OWASP kapcsolat

Ez elsősorban az A10:2025 exceptional-condition/resource-exhaustion kockázatok elleni védelem. Kiegészíti a HTTP body/header limiteket, bounded input típusokat, runtime allocation/instruction accountingot, request timeoutokat és process/cgroup ceilingeket; nem helyettesíti őket.
