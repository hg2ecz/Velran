<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Kritikus műveleti szerződések

A kritikus művelet egy névvel ellátott üzleti biztonsági szerződés. Egyszer deklaráljuk, milyen védelmek kötelezőek, a compiler pedig minden használatnál kikényszeríti őket.

```vrn
permission BillingWrite {
    role Admin
    role BillingAdmin
}

critical Payment {
    permission BillingWrite
    mfa
    transaction
    audit
}
```

A handler röviden csak a szándékot jelöli:

```vrn
#[action]
fn refund(ctx: ActionContext, db: Db, id: i64) -> Result<Json, PageError> critical Payment {
    transaction db {
        audit Payment id action refund;
    }
    return Ok(json(true));
}
```

A route sem ismétli meg a permission/MFA részleteket:

```vrn
route refund POST "/billing/:id<i64>/refund"
    auth critical Payment
    => refund;
```

Az `auth critical Payment` a szerződésből vezeti le a platform-owned auth policyt. Minden kritikus művelet legalább autentikációt követel; a permission és MFA ezt tovább szigorítja.

Ha a kötelező tranzakció vagy audit hiányzik, a program nem fordul le. Az audit továbbra sem fogad `Secret<T>` vagy nem megfelelően kezelt érzékeny adatot.
