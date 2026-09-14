<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 6. Statikus assetek, cache és precompressed fájlok

A Velran server külön, **read-only static rootból** tud CSS-t, képet, fontot és más build artifactot kiszolgálni.

## Indítás

```bash
velran-server \
  --app /srv/myapp/main.vrn \
  --static-root /srv/myapp/public \
  --static-url-prefix /assets/ \
  ...
```

Alapértelmezések:

```text
URL prefix                  /assets/
max asset                   8 MiB
regular max-age             300 s
fingerprinted max-age       31536000 s
precompressed .br/.gz       engedélyezett
```

A static root **nem azonos** az upload `--data-root` könyvtárral. Read-only `AppFs` nyitja meg, Linuxon ugyanazzal az `openat2` path/symlink confinementtel.

## Fájlnév és fingerprint

Ajánlott build output:

```text
public/
  css/app.01234567.css
  img/logo.a91f028c.svg
  fonts/ui-f1e2d3c4.woff2
```

A legalább 8 hex karakterből álló pont/kötőjel/underscore-del határolt token fingerprintnek számít. Ilyenkor:

```http
Cache-Control: public, max-age=31536000, immutable
```

Nem fingerprintelt fájlnál:

```http
Cache-Control: public, max-age=300, must-revalidate
```

A két TTL állítható:

```text
--static-max-age-secs <n>
--static-immutable-max-age-secs <n>
```

## ETag és conditional GET

Minden reprezentáció saját ETag-et kap:

```http
ETag: "velran-..."
```

A kliens:

```http
If-None-Match: "velran-..."
```

azonos tartalomnál `304 Not Modified` választ kap.

GET és HEAD támogatott. A static request **nem hoz létre sessiont és nem küld session cookie-t**.

## Brotli és gzip

A server nem tömörít request közben. A build pipeline készítse el:

```text
app.01234567.css
app.01234567.css.br
app.01234567.css.gz
```

`Accept-Encoding` alapján a sorrend:

```text
br
→ gzip
→ eredeti fájl
```

A válasz tartalmazza:

```http
Vary: Accept-Encoding
Content-Encoding: br
```

Ez szándékos: nincs requestenkénti compression CPU-költség vagy compression-DoS felület.

Ha a deploy nem generál precompressed fájlokat, semmi külön teendő nincs. Kikapcsolás:

```text
--no-precompressed-static
```

## MIME

Beépített MIME mapping van többek között CSS, JS/MJS, JSON/map, SVG, PNG/JPEG/GIF/WebP/AVIF, ICO, WOFF/WOFF2, TXT/XML és PDF számára. Ismeretlen extension:

```text
application/octet-stream
```

A server mindig küld `X-Content-Type-Options: nosniff` headert.

## Path szabályok

A static URL szándékosan szigorú. Például jó:

```text
/assets/css/app.01234567.css
```

Tiltott:

```text
../
%2e%2e
backslash
üres path komponens
space/control karakter
```

Percent-encoded static path v0.1-ben nincs; az asset build használjon URL-safe fájlneveket.

## Route namespace

A static prefix rezervált namespace. Ha `--static-url-prefix /assets/`, akkor az application nem deklarálhat vele ütköző route-ot. Olyan dinamikus első route segment is startup error, amely az `/assets/` namespace-t elfoghatná.

## CSP

A HTML response CSP same-origin asseteket enged:

```text
style-src 'self'
img-src 'self' data:
font-src 'self'
media-src 'self'
connect-src 'self'
```

A `default-src` továbbra is `'none'`; scriptet az M20 nem nyit meg automatikusan.
