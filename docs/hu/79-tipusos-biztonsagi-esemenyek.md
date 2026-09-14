<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Típusos biztonsági események

A Velran biztonsági eseményt egyszer kell deklarálni és egy modellhez kötni:

```vrn
security event RoleGranted for User;
```

Tranzakción belül az esemény röviden kibocsátható:

```vrn
security RoleGranted user.id;
```

Opcionálisan publikus, nem érzékeny állapotváltozás is rögzíthető:

```vrn
security RoleChanged user.id from oldRole to newRole;
```

`Secret<T>` és `Sensitive<T>` érték továbbra sem kerülhet a security event mezőibe. A critical operation konkrét eseményt is megkövetelhet:

```vrn
critical RoleChange {
    permission UserAdmin
    mfa
    transaction
    audit RoleGranted
}
```

Más audit vagy más security event nem teljesíti ezt a szerződést. Így a fejlesztői kód rövid marad, a kritikus állapotváltozás pedig nyelvi szinten megfigyelhető.
