<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 6. CRUD és tranzakciók

Mutáció csak `Transaction` capability-vel:

```text
#[query]
fn updateProduct(
    tx: Transaction,
    id: i64,
    name: String,
    price: i64
) -> Result<(), DbError> sql {
    UPDATE products
    SET name = :name, price = :price
    WHERE id = :id
}
```

Action:

```text
#[action]
fn update(
    ctx: ActionContext,
    db: Db,
    id: i64,
    name: String,
    price: i64
) -> Result<Redirect, PageError> {
    transaction db {
        updateProduct(tx, id, name, price)?
    }
    return Ok(redirect("/products?page=1&pageSize=20"));
}
```

Siker → commit. Query/runtime hiba → rollback. A `Transaction` nem escape-elhet a blokkból.

Hordozható SQLite/PostgreSQL/MariaDB CRUD-nál a `Result<(), DbError>` jó közös minimum; backend-specifikus `RETURNING` csak tudatosan.
