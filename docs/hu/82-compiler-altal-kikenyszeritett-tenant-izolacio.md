<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Compiler által kikényszerített tenant izoláció

A Velran tenant izolációja a platform által kezelt, hitelesített membership claimre épül. A tenant scope nem egyszerű String-összehasonlítás, és nem az első membershipből következtetjük ki.

## Scoped modell

```vrn
type OrganizationId = String {
    length 1 64;
    pattern "^[a-z0-9_-]+$";
}

model Article scoped by organizationId {
    id: i64
    organizationId: OrganizationId
    title: String
}
```

A `scoped by organizationId` a tenant mezőt a modell biztonsági szerződésének részévé teszi. A mezőnek léteznie kell és String reprezentációjúnak kell lennie; erre nominális domain típus ajánlott.

## Aktív tenant route proof

```vrn
route article GET "/org/:organizationId<OrganizationId>/articles/:id<i64>"
    tenant organizationId
    auth user
    => article;
```

A route nem lehet publikus. A runtime a handler futása előtt ellenőrzi, hogy a kiválasztott tenant szerepel-e a trusted `__authMemberships` session claimben. Tagság nélkül `Forbidden` az eredmény.

Tenant csak path paraméterből vagy GET query paraméterből választható. POST body nem lehet tenant authority, ezért az izoláció már a request body feldolgozása előtt létrejön.

## Query enforcement

Scoped modellt visszaadó vagy módosító query automatikusan tenant-scoped; nincs külön query annotáció:

```vrn
#[query]
fn loadArticle(
    db: Db,
    organizationId: OrganizationId,
    id: i64
) -> Result<Article, DbError> sql {
    SELECT id, organizationId, title
    FROM articles
    WHERE organizationId = :organizationId
      AND id = :id
}
```

A compiler megköveteli:

- a modell tenant mezőjével azonos nevű query paramétert;
- pontos nominális típusazonosságot;
- SELECT/UPDATE/DELETE esetén SQL tenant guardot (`field = :field`);
- INSERT esetén megfelelő oszlop/érték párt;
- a hívási helyen aktív-route-tenant proofot hordozó argumentumot.

Egy közönségesen validált `OrganizationId` nem elegendő. A route `tenant organizationId` bindingjából kell származnia és változatlanul továbbadni.

## Biztonsági tulajdonságok

A modell több gyakori cross-tenant IDOR/BOLA hibát kizár:

- elfelejtett tenant SQL predicate → compile error;
- másik organization ID átadása → compile error;
- publikus tenant route → compile error;
- POST bodyból származó tenant authority → compile error;
- runtime membership-check a handler előtt.

A rendszer szándékosan fail-closed. Több scoped modellt érintő cross-tenant műveletet egyelőre külön scope-olt lépésekre kell bontani.
