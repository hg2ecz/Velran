<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 11. Date, DateTime, Uuid és Decimal

Az M26 négy first-class üzleti típust ad a nyelvhez:

```text
Date
DateTime
Uuid
Decimal
```

Használhatók model mezőben, page/action/query paraméterben, path/query/form/JSON inputban, typed SQL bindben, HTML-ben és JSON outputban.

## Date

Canonical formátum:

```text
2026-09-04
```

Példa:

```text
route report GET "/reports/:day<Date>" public => report;
```

Érvénytelen naptári dátum, például `2026-02-30`, `400 Bad Request`.

## DateTime

Input RFC 3339:

```text
2026-09-04T10:30:00+02:00
```

A runtime UTC-ra normalizálja. Canonical output:

```text
2026-09-04T08:30:00Z
```

Timezone nélküli datetime nem elfogadott; így nincs implicit local-time értelmezés.

## Uuid

```text
550e8400-e29b-41d4-a716-446655440000
```

A runtime validálja és canonical hyphenated lowercase formában rendereli.

## Decimal

Pénzügyi és pontos decimális számhoz `Decimal`-t használj, ne floatot.

```text
19.99
-1200.125
```

`Decimal` támogatja:

```text
+
-
*
/
```

csak `Decimal` és `Decimal` között. Nincs implicit `i64`→`Decimal` coercion.

Példa:

```text
#[action]
fn calculate(ctx: ActionContext, net: Decimal, tax: Decimal)
    -> Result<Json, PageError> {
    let gross = net + tax;
    return Ok(json(gross));
}
```

## JSON contract

A `Date`, `DateTime`, `Uuid` és `Decimal` JSON-ban **string**. Ez különösen `Decimal` esetén szándékos, mert a JSON/JavaScript number nem garantál pontos decimális reprezentációt.

```json
{
  "issued": "2026-09-04",
  "createdAt": "2026-09-04T08:30:00Z",
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "price": "19.99"
}
```

Typed JSON inputnál ugyanilyen stringet küldj.

## DB mapping

A hordozható v0.1 contract canonical text representation:

| Velran | portable DB representation |
|---|---|
| `Date` | `TEXT` / `VARCHAR` `YYYY-MM-DD` |
| `DateTime` | `TEXT` / `VARCHAR` RFC3339 UTC |
| `Uuid` | `TEXT` / `VARCHAR` canonical UUID |
| `Decimal` | `TEXT` / `VARCHAR` canonical decimal |

Példa portable schema:

```sql
CREATE TABLE invoices (
    id TEXT PRIMARY KEY,
    issued TEXT NOT NULL,
    created_at TEXT NOT NULL,
    net TEXT NOT NULL
);
```

A runtime a DB-ből visszaolvasott értéket újra típusvalidálja. Hibás canonical adat `Database` hibát eredményez, nem lesz csendben `String`.

A text mapping szándékosan stabil SQLite/PostgreSQL/MariaDB között. Később backend-specifikus natív DATE/TIMESTAMP/UUID/NUMERIC mapping hozzáadható úgy, hogy a nyelvi contract ne változzon.

## HTML és URL

A négy típus escaped textként renderelhető és typed route argumentumként használható. URL-ben canonical text kerül percent-encodingra.

## Biztonsági szabályok

- pénzhez ne használj lebegőpontot;
- DateTime mindig timezone-os RFC3339 input;
- DB-ből érkező canonical érték is újravalidált;
- UUID nem authority: attól, hogy egy ID formailag valid, authorization továbbra is szükséges;
- Decimal division by zero fail-closed runtime error.

## Slug

M36-tól a keresőbarát URL-ekhez first-class `Slug` használható. A portable DB representation canonical text; route/path/query/form/JSON inputnál ugyanaz a validáció fut.

```velran
model Article {
    slug: Slug
    title: String
}
```

Slug készítéshez `slug(title)` használható. Részletek: [Projektstruktúra, modulok és keresőbarát URL-ek](21-modules-slugs-project-layout.md).
