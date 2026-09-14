<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Függvényszintű jogosultságok

A Velran permission névvel ellátott alkalmazási képesség, amelyet a platform által kezelt autentikált session meglévő role-jai támasztanak alá. Így az alkalmazáskód üzleti szándékot fejez ki, nem role-listákat másol route-ok és handlerek között.

## A permission egyszer deklarálandó

```vrn
permission UserAdmin {
    role Admin
    role SecurityAdmin
}
```

A felsorolt role-ok alternatívák: bármelyik megléte megadja a permissiont.

## Handler-szintű szerződés

```vrn
#[action]
fn removeUser(
    ctx: ActionContext,
    id: i64
) -> Result<Json, PageError> requires UserAdmin {
    return Ok(json(true));
}
```

A `requires UserAdmin` függvényszintű biztonsági szerződés. Ezt a handlert nem lehet pusztán `auth user`, `auth mfa`, nem megfelelő role vagy `public` route mögé kötni.

A normál happy path rövid:

```vrn
route removeUser POST "/users/:id<i64>/remove"
    auth permission UserAdmin
    => removeUser;
```

A compiler feloldja a permissiont, a szerver pedig a jelenlegi session role-jai alapján engedélyez. Az alkalmazáskódnak nem kell role stringeket vizsgálnia.

## Migráció nyers role-ról

Átmenetileg elfogadott az a route is, amely közvetlenül olyan role-t követel, amely explicit módon megadja a handler permissionjét:

```vrn
route removeUser POST "/users/:id<i64>/remove"
    auth role Admin
    => removeUser;
```

Új kódban az `auth permission UserAdmin` ajánlott, mert az üzleti jogosultsági szándék akkor is stabil marad, ha a role-hozzárendelések később változnak.

## Fail-closed szabályok

A Velran elutasítja:

- az ismeretlen permissiont handleren vagy route-on;
- a permissiont igénylő handler `public` vagy sima `auth user` kitettségét;
- a permissiont nem biztosító role-t;
- az üres permission deklarációt;
- a duplikált role-t ugyanabban a permissionben.

Fő diagnosztikák:

- `SEC-A01-020`: a handler ismeretlen permissiont kér;
- `SEC-A01-021`: a route ismeretlen permissionre hivatkozik;
- `SEC-A01-022`: a route auth nem elégíti ki a handler permission-szerződését.

A permission compile-time domain fogalom, miközben runtime backingként a meglévő session role-lista marad. Így nem jön létre második identity rendszer, és a fejlesztő továbbra is rövid, emberi kódot ír.
