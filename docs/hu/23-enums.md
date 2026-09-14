<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Enumok — zárt üzleti állapotok

Ügyviteli vagy CMS alkalmazásban a véges állapotokat ne szabad `String` mezővel modellezd.

Rosszabb:

```velran
status: String
```

Ajánlott:

```velran
enum ArticleStatus {
    Draft
    Review
    Published
    Archived
}

object Article {
    model {
        id: i64
        status: ArticleStatus
    }
}
```

## Literál

```velran
let next = ArticleStatus::Published;
```

A compiler ellenőrzi a típust és a variáns nevét. Ez hibás:

```velran
let next = ArticleStatus::Publshed;
```

## Query

```velran
#[query]
fn setStatus(
    tx: Transaction,
    id: i64,
    status: ArticleStatus
) -> Result<(), DbError> sql {
    UPDATE articles
    SET status = :status
    WHERE id = :id
}
```

Majd:

```velran
let status = ArticleStatus::Published;
transaction db {
    Article.setStatus(tx, id, status)?;
}
```

## Route, form és JSON

Enum route/form/JSON scalar típusként is használható:

```velran
route list GET "/articles"
    query status<ArticleStatus>
    public => Article.list;
```

Wire formátuma a variáns pontos neve, például `Published`. A `published`, `PUBLISHED` vagy ismeretlen érték inline route/query/JSON contractnál `400 Bad Request`; named form mezőjeként mezőszintű `invalid_type` hibává és `422 Unprocessable Content` válasszá válik.

Named form validation hibaoldalnál a szerver az enumot `<select>` mezőként rendereli, kizárólag a compiler által ismert variánsokkal.

## JSON

JSON input és output canonical string:

```json
{"status":"Published"}
```

Nem használunk integer ordinalt. Így az enum deklaráció sorrendjének későbbi megváltoztatása nem írja át észrevétlenül az adat jelentését.

## Adatbázis

A portable mapping `TEXT/VARCHAR`. Példa:

```sql
status TEXT NOT NULL
```

A runtime minden DB-ből visszaolvasott értéket újra validál az enum deklaráció ellen. Hibás adat fail-closed database error.

Adatbázis oldalon is ajánlott CHECK constraintet használni, ha a backend és migration policy ezt lehetővé teszi:

```sql
CHECK (status IN ('Draft', 'Review', 'Published', 'Archived'))
```

Ez defense in depth; a nyelvi ellenőrzést nem helyettesíti.

## Elnevezés nagyobb projektben

M38-ban az enum top-level domain type. Domain-specifikus, beszédes nevet használj:

```text
ArticleStatus
InvoiceState
UserLifecycle
PaymentStatus
```

Az object továbbra is a műveleti namespace:

```velran
Article.publish(...)
```

Az enum pedig a domain állapottípus:

```velran
ArticleStatus::Published
```

Ezzel nincs szükség globális string-konvenciókra, és a code review-ban az üzleti állapotváltások jól látszanak.
