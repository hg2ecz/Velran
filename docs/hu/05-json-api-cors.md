<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# JSON API és CORS

Az M19-től ugyanabban a Velran alkalmazásban HTML oldalak és typed JSON endpointok is készíthetők.

## JSON válasz

Scalar:

```text
#[page]
fn statusApi(ctx: PageContext) -> Result<Json, PageError> {
    let healthy = true;
    return Ok(json(healthy));
}

route statusApi GET "/api/status" public => statusApi;
```

DB modell vagy lista:

```text
#[page]
fn productsApi(
    ctx: PageContext,
    db: Db,
    limit: i64,
    offset: i64
) -> Result<Json, PageError> {
    let products = listProducts(db, limit, offset)?;
    return Ok(json(products));
}

route productsApi GET "/api/products"
    query limit<i64> offset<i64>
    validate limit range 1 100 offset range 0 1000000
    public => productsApi;
```

`json(value)` támogatott runtime értékei: `String`, `i64`, `bool`, model record, optional model (`null`) és modell-lista.

## Typed JSON POST body

```text
#[action]
fn echoApi(
    ctx: ActionContext,
    name: String,
    age: i64,
    active: bool
) -> Result<Json, PageError> {
    return Ok(json(name));
}

route echoApi POST "/api/echo"
    json name<String> age<i64> active<bool>
    validate name length 1 100 age range 0 150
    auth user
    => echoApi;
```

Request:

```http
POST /api/echo HTTP/1.1
Content-Type: application/json
Accept: application/json
X-CSRF-Token: <session CSRF token>

{"name":"Alice","age":42,"active":true}
```

A JSON schema zárt. Hibás request:

- ismeretlen kulcs;
- hiányzó kulcs;
- duplikált kulcs;
- rossz scalar típus;
- float, `null`, array vagy nested object inputként;
- mező/érték méretlimit túllépés.

A `json`, `form` és `upload` ugyanazon POST route-on egymást kizáró body módok.

## Content negotiation

JSON response:

```text
Content-Type: application/json; charset=utf-8
```

Az `Accept` header támogatja az exact media type-ot, a type wildcardot és `*/*`-ot. Például:

```text
Accept: application/json
Accept: application/*
Accept: */*
```

Ha a handler JSON-t adna, de a kliens csak `text/html`-t fogad el, a szerver `406 Not Acceptable` választ ad.

## API hibák

JSON-return típusú route-ok a route felismerése után JSON error envelope-ot kapnak:

```json
{"error":"bad_request"}
```

Tipikus kódok: `unauthorized`, `forbidden`, `csrf_failed`, `bad_request`, `unsupported_media_type`, `database_unavailable`, `resource_limit`, `internal_error`. Autholt JSON route unauthenticated kérésnél `401`-et kap, nem HTML login redirectet.

## CSRF JSON API-n

Form POST:

```text
_csrf=<token>
```

JSON POST:

```text
X-CSRF-Token: <token>
```

A token ugyanahhoz a sessionhöz tartozik. CORS nem helyettesíti a CSRF ellenőrzést.

Ha külön frontend origin használ session authot, készíthető védett token endpoint:

```text
#[page]
fn csrfApi(ctx: PageContext) -> Result<Json, PageError> {
    return Ok(json(csrfToken));
}

route csrfApi GET "/api/csrf" auth user => csrfApi;
```

## CORS

Alapból cross-origin response nincs engedélyezve.

Exact origin allowlist:

```bash
--cors-origin https://frontend.example
```

Több originhez ismételd meg a flaget. `*` wildcard nincs.

Session-cookie-s cross-origin frontendhez:

```bash
--cors-origin https://frontend.example \
--cors-allow-credentials
```

Ez csak HTTPS production módban engedélyezett. A session cookie ekkor `SameSite=None; Secure`.

A preflight csak:

- konfigurált Originre;
- létező GET/POST route-ra;
- `Content-Type`, `X-CSRF-Token`, `Accept` request headerekre

engedélyezett.

Frontend példa:

```javascript
const csrf = await fetch("https://api.example/api/csrf", {
  credentials: "include",
  headers: { Accept: "application/json" }
}).then(r => r.json());

const result = await fetch("https://api.example/api/echo", {
  method: "POST",
  credentials: "include",
  headers: {
    "Content-Type": "application/json",
    Accept: "application/json",
    "X-CSRF-Token": csrf
  },
  body: JSON.stringify({ name: "Alice", age: 42, active: true })
}).then(r => r.json());
```

## Teljes példa

Lásd: `examples/json-api/app.vrn`.
