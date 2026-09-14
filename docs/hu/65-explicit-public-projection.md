<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Explicit publikus projection

A Velran nem engedi, hogy egy betöltött modell implicit módon JSON response-ba kerüljön. A publikus API-kimenet explicit mezőlista:

```vrn
return Ok(json(expose(user, id, displayName)));
```

Az `expose(value, field, ...)` modellre, opcionális modellre vagy modelllistára alkalmazható, és legalább egy mezőt meg kell nevezni. Ismeretlen vagy duplikált mező fordítási hiba. A runtime kizárólag a felsorolt mezőket adja ki, ezért egy új modellmező hozzáadása nem bővíti csendben a már létező API-választ.

## Adatklasszifikáció

A `Public` mezők közvetlenül kivetíthetők. `Sensitive<T>` mezőhöz az adott betöltött objektum authorization proofja szükséges. `Secret<T>` mező soha nem tehető publikus response-ba, autorizáció után sem.

```vrn
model User {
    id: UserId
    email: Sensitive<Email>
    passwordHash: Secret<String>
    owner: String
}

let user = loadUser(db, id)?;
authorize user owner owner;
return Ok(json(expose(user, id, email)));
```

A `passwordHash` kivetítése `SEC-DATA-005`, az `email` autorizáció nélküli kivetítése `SEC-A01-012` hibát ad.

## Modelllisták

Ugyanez a szintaxis egy lista minden elemére alkalmazza a projectiont:

```vrn
let products = listProducts(db)?;
return Ok(json(expose(products, id, name, price)));
```

Így nincs teljes rekord szerializáció, és code review során pontosan látszik a publikus response szerződése.

## Biztonsági invariáns

A közvetlen `json(user)` vagy `json(products)` tiltott (`SEC-DATA-004`). Az `expose` tudatosan response-boundary konstrukció, nem általános expression: belső kódban nem lehet vele egyszerűen eltüntetni az adat érzékenységi besorolását.
