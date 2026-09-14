<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# MFA-elevációs proofok

A Velran a handler határán képes kötelezővé tenni az MFA-elevációt anélkül, hogy az alkalmazáskód proof-wrapper típusokat vagy session plumbingot kezelne.

## Csak MFA-t igénylő érzékeny handler

```vrn
#[action]
fn disableMfa(
    ctx: ActionContext
) -> Result<Json, PageError> requires mfa {
    return Ok(json(true));
}

route disableMfa POST "/account/mfa/disable"
    auth mfa
    => disableMfa;
```

A compiler elutasítja a gyengébb `auth user` route-ot. A runtime a platform által kezelt session `mfa_verified` állapotát használja; az alkalmazás nem épít második MFA-rendszert.

## Permission + MFA együtt

```vrn
permission BillingWrite {
    role Admin
    role BillingAdmin
}

#[action]
fn refund(
    ctx: ActionContext,
    id: i64
) -> Result<Json, PageError> requires BillingWrite + mfa {
    return Ok(json(true));
}

route refund POST "/billing/:id<i64>/refund"
    auth permission BillingWrite mfa
    => refund;
```

Runtimeban mindkét feltétel kötelező: a sessionnek rendelkeznie kell a `BillingWrite` permissiont megadó role-lal és MFA-verified állapotban kell lennie.

## Fail-closed szabály

A `requires ... + mfa` nem dokumentáció, hanem handler-szintű biztonsági szerződés. Ha a route megadja a permissiont, de kihagyja az MFA-t, a fordítás `SEC-A07-020` hibával megáll.

A biztonságos happy path szándékosan rövid: a fejlesztő az intentet deklarálja, a compiler és a platform session pedig kikényszeríti a mechanikát.
