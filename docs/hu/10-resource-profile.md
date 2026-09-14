<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 10. Resource profile-ok

A normál kód kis soft budgettel futhat, a számításigényes rész pedig egy **operátor által definiált** profilt kérhet. A kód nem adhat meg saját számértékeket.

```text
#[page]
fn report(ctx: PageContext) -> Result<Html, PageError> {
    with resource compute {
        let result = 40 + 2;
        return Ok(html {<p>{{ result }}</p>});
    }
}
```

A `compute` nem beépített név; bármilyen statikus identifier használható, ha a szerver configja definiálja.

## Konfiguráció

`profiles.toml`:

```toml
[profile.default]
max_instructions = 100000
max_alloc_bytes = 33554432
max_concurrent = 4096

[profile.compute]
max_instructions = 2000000
max_alloc_bytes = 134217728
max_concurrent = 2
```

Productionban a request hard ceilingeket és a profile fájlt a trusted server config adja meg; a `.vrn` kód csak a profile nevét kérheti. A CLI megfelelő opciói célzott override/development célra maradnak.

A request hard ceilingeket egy named profile nem lépheti túl.

## Végrehajtás

Minden művelet egyszerre fogyasztja:

```text
request hard budget
+
aktuális scope/profile budget
```

A profile-váltás nem tölti vissza a request keretet. A `max_concurrent` külön semaphore, ezért például csak két `compute` blokk futhat egyszerre.

## Startup audit

A szerver induláskor minden profile-kérést egyszer naplóz:

```json
{"event":"resource_profile_use","file":"src/reports.vrn","line":84,"function":"monthlyReport","profile":"compute","max_instructions":2000000,"max_alloc_bytes":134217728,"max_concurrent":2,"elevated":true}
```

Ismeretlen profil esetén a szerver nem indul el.

## v0.1 korlátozás

A `with resource ... {}` blokk a page/action **utolsó blokkja**, és nested resource profile nincs. A blokkban lehet a számítás és a végső `return`. Ez szándékosan egyszerű, jól auditálható v0.1 szemantika.

A profile csak instruction/allocation soft budgetet emelhet. DB-, hálózat-, filesystem-, upload-, cgroup- vagy process-limitet nem.
