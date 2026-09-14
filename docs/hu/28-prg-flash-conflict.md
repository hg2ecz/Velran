<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# PRG, flash üzenetek és 409 conflict UX

Normál HTML form mentésnél az ajánlott út a Post/Redirect/Get minta.

```velran
#[action]
fn saveArticle(ctx: ActionContext) -> Result<Redirect, PageError> {
    // üzleti módosítás / transaction
    flash success "Article saved";
    return Ok(redirect("/articles"));
}
```

A `redirect(...)` sikeres POST actionből `303 See Other`. A cél GET oldalon:

```velran
#[page]
fn articles(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {
        <main>
            @flash()
            <h1>Articles</h1>
        </main>
    });
}
```

## Miért statikus a flash szövege?

A flash UX-elem, nem általános session storage. M44-ben a szöveg compiler-owned literal. Így request body, DB-ből olvasott PII vagy secret nem kerül véletlenül session-backed átmeneti állapotba.

Támogatott típusok:

```velran
flash success "Saved";
flash info "Draft kept";
flash warning "Review required";
flash error "Operation could not be completed";
```

Actionönként legfeljebb egy flash állítható, és csak Redirect-return mellett.

## Egyszer használatos

A runtime a flash-t a sessionhöz köti. A következő GET elfogyasztja. Redis production backendnél az olvasás+törlés atomikus tranzakció, ezért ugyanazt az üzenetet normál esetben nem lehet kétszer kiolvasni.

A flash-t ténylegesen tartalmazó HTML válasz `Cache-Control: no-store`. `@flash()`-tól függő oldal public cache-t nem használhat.

## 409 Conflict

Optimistic locking vagy DB UNIQUE conflict esetén Velran nem írja felül automatikusan az újabb adatot.

HTML route:

- HTTP 409;
- emberi, statikus conflict oldal;
- azt kéri, hogy a user töltse újra az edit oldalt és nézze át az aktuális értékeket;
- `Cache-Control: no-store`.

JSON API:

```json
{"error":"conflict"}
```

A DB constraint neve, SQL szöveg vagy driver error nem kerül a klienshez.

## Form validation és conflict nem ugyanaz

A 422 form validation azt jelenti, hogy a beküldött input nem felel meg a deklarált form-szerződésnek. A 409 azt jelenti, hogy az input önmagában lehetett helyes, de a tárolt állapot időközben megváltozott vagy DB-authoritative uniqueness konfliktus történt.

Ezt a két hibát ne mosd össze előzetes `SELECT`-alapú „unique validációval”.
