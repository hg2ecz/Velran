<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Autorizált mutációk

A Velran az objektumazonosságot az `UPDATE` és `DELETE` műveletek autorizációs bizonyítékának részévé teszi.

A módosító query deklarálja, mely modellt és kulcsot módosítja:

```velran
#[query]
fn updateArticle(
    tx: Transaction,
    id: i64,
    title: String
) -> Result<(), DbError> mutates Article by id sql {
    UPDATE articles SET title = :title WHERE id = :id
}
```

`UPDATE` vagy `DELETE` `mutates` szerződés nélkül `SEC-A01-006` fordítási hibát ad.

A hívásban a kulcsnak autorizált modellpéldányból kell származnia:

```velran
let article = articleById(db, id)?;
authorize article owner authorUsername or role Editor;

transaction db {
    updateArticle(tx, article.id, title)?;
}
```

A requestből érkező `id` közvetlen átadása `SEC-A01-005` hibát ad. Egy változatlan lokális alias megőrzi a proofot, egy transzformáció (`article.id + 1`) viszont szándékosan elveszíti.

Más modellből származó proof azonos skalártípus esetén sem használható fel.

## Közös objektumok

Ha egy objektumot szándékosan bármely bejelentkezett felhasználó kezelhet, ez explicit leírható:

```velran
authorize product authenticated;
```

Így nem kell mesterséges owner mezőt létrehozni, de az engedélyezés továbbra sem implicit.

## Frissen létrehozott objektumok

Az `INSERT ... RETURNING` által visszaadott modell friss objektumként mutációs proofot kap ugyanabban a flow-ban. Ez lehetővé teszi a tömör create-then-update tranzakciókat anélkül, hogy requestből származó azonosító automatikusan jogosultsággá válna.
