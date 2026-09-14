<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Domain-validáció és tiszta input szerződések

A Velranban a validáció elsődleges célja nem az, hogy minél több szabályt lehessen egy DSL-be írni, hanem hogy a handler már megbízható, típusos adatot kapjon.

## Email mint domain típus

Ha egy mező email-cím, ne `String` + ismételt email-validator legyen:

```velran
form ContactForm {
    email<Email>
}
```

A handlerben is látszik a szerződés:

```velran
#[action]
fn contactSave(ctx: ActionContext, email: Email) -> Result<Json, PageError> {
    return Ok(json(email));
}
```

Az `Email` használható modelben, query paraméterként, route/form/JSON inputként és typed URL értékként is.

### Canonical forma

A runtime:

- legfeljebb 254 ASCII byte-ot fogad;
- tiltja a whitespace/control karaktereket;
- pontosan egy `@` elválasztást követel;
- ellenőrzi a local-part dot szabályait és az internetes domain label-eket;
- a domain részt kisbetűsíti;
- a local partot változatlanul hagyja.

Példa:

```text
User::Name+news@Example::COM
→ User::Name+news@example.com
```

A DB-ben már canonical értéket kell tárolni. Hibás vagy nem canonical adat DB integrity hibaként fail-closed.

> A Velran itt tudatosan konzervatív, ASCII e-mail surface-t ad. Nem próbálja meg teljes SMTP/RFC címparserként kezelni a display-name, quoted local-part vagy nemzetközi mailbox összes esetét. Ha ilyen címek üzleti követelmények, azt külön domain feature-ként kell bővíteni, nem lazább `String` ellenőrzéssel.

## Cross-field validáció

Két mező kapcsolatát ne új típussal modellezd. Erre való a `same`:

```velran
form ContactForm {
    email<Email>
    confirmEmail<Email>
    validate confirmEmail same email
}
```

A compiler ellenőrzi, hogy:

- mindkét mező létezik;
- a két típus pontosan azonos;
- `Upload` és `Image` nem használható `same` szabállyal.

A runtime előbb typed decode-ot végez, utána hasonlítja össze a két értéket. Named form esetén eltéréskor `422 Unprocessable Content` és mezőhiba (`same`) jön, a korábbi értékek megtartásával.

## URL mint domain típus

Általános webes URL-hez használj `Url` típust, ne ad-hoc `String` ellenőrzést:

```velran
form LinkForm {
    target<Url>
}
```

A `Url` csak abszolút `http`/`https` URL-t fogad hosttal, tiltja a beágyazott user/password credentialt és canonical formára normalizál. A DB-ben canonical értéket vár. Fontos: a `Url` érték **nem hálózati capability**; outbound fetch továbbra is csak named target policy alapján történhet.

## Pattern validáció

Ha a követelmény ténylegesen egy lokális string-formátum, használható a szűk `pattern` szabály:

```velran
form VoucherForm {
    code<String>
    validate code pattern "^[A-Z]{3}-[0-9]{4}$"
}
```

A szabály csak `String` mezőn engedélyezett. A minta compiler-owned statikus literál, legfeljebb 256 byte, és a compiler már fordításkor ellenőrzi. Closed választási halmazra ne regexet és ne `oneOf`-ot használj, hanem first-class `enum`-ot.

## Optional és oneOf

M42 szándékosan nem vezet be külön `oneOf` validátort: ezt erősebben lefedi az `enum`. Form-only `optional` flag sincs, mert a valódi optional inputnak a handler típusában és a runtime/wire szerződésben is látszania kell; félmegoldásként nem adunk `Null`-t nem opcionális paraméterhez.

## Unique validáció

A uniqueness nem lehet kizárólag form-validator:

```text
request A: nincs ilyen email
request B: nincs ilyen email
request A: INSERT
request B: INSERT
```

Ezért az authority a DB `UNIQUE` constraint. M42-től a mutating transaction queryből érkező DB unique-violation `409 Conflict` lesz, miközben más DB-hiba továbbra is generikus adatbázishiba. A konkrét mezőhöz kötött emberi hibaüzenet a PRG/flash/conflict UX réteg feladata.

## Clean-code ajánlás

A fő szabály:

```text
egy érték saját invariánsa  → domain type
több mező kapcsolata       → validation rule
adat-integritási invariáns  → DB constraint
jogosultsági invariáns      → authorization
```

Ez rövidebb handlereket, jobb code review-t és kevesebb duplikált validációt ad.
