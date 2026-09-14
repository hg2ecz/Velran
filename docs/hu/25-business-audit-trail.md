<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Üzleti audit trail

Az access/security log és az üzleti változástörténet két külön dolog. A Velran M40 az alkalmazás adatbázisában tartós, tranzakciós business audit rekordot tud létrehozni.

## Mikor használd?

Akkor, amikor a művelet üzleti jelentése később rekonstruálandó, például:

- cikk publikálása vagy archiválása;
- számla jóváhagyása;
- rendelés állapotváltása;
- jogosultságon túli üzleti workflow döntés.

Normál oldalmegtekintést ne business auditálj; arra az access log való.

## Alapminta

```velran
enum ArticleStatus {
    Review
    Published
}

transaction db {
    Article.publishChanged(tx, id, version)?;
    audit Article id action publish from article.status to ArticleStatus::Published;
}
```

Az audit ugyanabban a DB-tranzakcióban fut, mint a módosítás. Ha az audit rekord nem írható be, a business módosítás is rollbackel.

## Rövid esemény auditja

Ha nincs értelmes `from/to` állapot:

```velran
transaction db {
    Invoice.send(tx, id)?;
    audit Invoice id action send;
}
```

## Tárolt mezők

A runtime a `_velran_business_audit` táblába írja:

- `event_id` — runtime-generált UUID;
- `occurred_at` — UTC RFC3339 timestamp;
- `request_id` — HTTP request korreláció;
- `actor` — canonical autentikált principal;
- `source_action` — Velran action neve, pl. `Article.publish`;
- `object_type` — statikus domain/model név;
- `object_id` — canonical objektumazonosító;
- `action` — statikus üzleti eseménynév;
- `previous_value`, `new_value` — opcionális rövid domain állapot.

A séma példája: `examples/business-audit/migrations/0001_business_audit.sql`.

## Adatminimalizálás

A `from/to` érték nem lehet szabad `String`, `Image` vagy `Upload`. Használj typed enumot, bool/i64/Date/DateTime/Uuid/Decimal/Slug értéket. Ez szándékosan megakadályozza form-body, megjegyzés, cím vagy más nagy/szenzitív szöveg véletlen audit-dumpját.

Az objektumazonosító lehet String is, mert legacy/business kulcsoknál erre szükség lehet, de runtime hard limit vonatkozik rá.

## Auth és authority

Business auditot tartalmazó action nem tehető public route mögé. Az actor a session canonical principalja; kliens által küldött `username`, `role` vagy `actor` mező nem authority.

Az audit statement authorizationt nem helyettesít:

```velran
let invoice = Invoice.byId(db, id)?;
authorize invoice owner ownerUsername or role Accountant;
transaction db {
    Invoice.approve(tx, id, version)?;
    audit Invoice id action approve from invoice.status to InvoiceStatus::Approved;
}
```

## `_velran_` reserved DB namespace

Az alkalmazás `.vrn` queryje nem hivatkozhat `_velran_` nevű runtime táblára. A compiler ezt elutasítja. Így az app query nem tudja saját jogán átírni vagy törölni a business audit trailt.

A migration CLI természetesen létrehozhatja a runtime táblát.

Ez nem jelent védelmet a DB admin vagy a DB-fájlt közvetlenül módosító operator ellen. Erős compliance környezetben a DB audit trail mellett a meglévő `audit.log` központi, append-only/SIEM továbbítása továbbra is ajánlott.

## Optimistic lockinggel együtt

Az ajánlott sorrend:

```velran
transaction db {
    Article.publishChanged(tx, id, version)?  // Changed: 0 sor => 409
    audit Article id action publish from article.status to ArticleStatus::Published;
}
```

Stale verziónál az első statement `409 Conflict` hibát ad, az audit statement nem fut le. Így sikertelen változtatásból nem lesz hamis success audit rekord.
