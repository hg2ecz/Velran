<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Biztonságos képek és media library

Az M33 első media surface-e a képfeltöltést külön typed értékként kezeli. Célja, hogy CMS, wiki és hírportál kód ne nyers fájlpathot vagy kliens által megadott MIME típust használjon.

## Képfeltöltés

```velran
#[action]
fn saveHero(ctx: ActionContext, hero: Image) -> Result<Json, PageError> {
    return Ok(json(hero));
}

route saveHero POST "/admin/hero"
    upload hero<Image> to "media" publish
    auth user
    => saveHero;
```

Az `Image` upload a már meglévő streaming multipart + CSRF + AppFs útvonalat használja. A célkönyvtár statikus, relatív, URL-safe AppFs útvonal. A fájlnév szerver által generált, ezért a kliens fájlneve nem lesz storage path.

## Mi számít képnek?

A szerver nem bízik a multipart `Content-Type` mezőben. A tárolt fájlt magic byte és struktúra alapján ellenőrzi.

M33-ban engedélyezett:

- PNG;
- JPEG.

Szándékosan tiltott az SVG, mert aktív tartalmat és script/XSS felületet hordozhat. GIF és WebP még nincs engedélyezve; ezek külön validátorral adhatók hozzá később.

A `storage.max_image_pixels` operátori hard limit a `width * height` értéket korlátozza. Alapértelmezés: 40 000 000 pixel. A byte-limitet továbbra is `storage.max_upload_bytes` korlátozza.

```toml
[storage]
data_root = "/srv/velran/data"
fs_mode = "rwc"
max_upload_bytes = 16777216
max_image_pixels = 40000000
```

Image uploadhoz az AppFs `rwc` jogosultsága szükséges: `c/w` az atomikus feltöltéshez és rollbackhez, `r` a szerveroldali képellenőrzéshez és kiszolgáláshoz.

## Tárolható typed referencia

Az `Image` first-class scalar. DB-ben canonical textként tárolható, ugyanúgy portábilisan SQLite/PostgreSQL/MariaDB alatt, majd `Image` model mezőként visszaolvasható.

Ne bontsd fel és ne gyártsd kézzel a canonical értéket. A szerver állítja elő validált feltöltés után.

## Renderelés

```velran
#[page]
fn article(ctx: PageContext, article: Article) -> Result<Html, PageError> {
    return Ok(html {
        <article>
            <h1>{{ article.title }}</h1>
            @image(article.hero, article.title)
        </article>
    });
}
```

`@image(image, alt)`:

- első paramétere `Image`;
- második paramétere `String`;
- csak HTML content pozícióban használható;
- automatikusan escape-eli az alt szöveget;
- kiírja a validált `width`/`height` attribútumot;
- `loading="lazy"` és `decoding="async"` attribútumot használ;
- a `src` kizárólag a beépített read-only media endpoint lehet.

Az `Image` közvetlen `{{ image }}` interpolációja compiler error. Ez megakadályozza, hogy a canonical media referencia véletlenül URL-ként vagy markupként legyen kezelve.

## Media endpoint

A szerver fenntartja a következő prefixet:

```text
/__velran/media/
```

Az alkalmazás nem deklarálhat ezzel ütköző route-ot. A media endpoint csak olyan AppFs könyvtárból szolgál ki, amelyet a program valamely `upload ...<Image> to "..." publish` route-ja deklarált. A teljes data root soha nem válik HTTP-n böngészhetővé.

A kiszolgáláskor a fájlt újra ellenőrzi a képdetektor, és a MIME típust ebből állapítja meg. A válasz `nosniff` védelmet, immutable cache headert és ETag-et kap. Csak GET/HEAD támogatott.

## Biztonsági modell

- kliens MIME típusa nem authority;
- kliens fájlneve nem storage path;
- symlink/hardlink/path traversal elleni AppFs szabályok változatlanok;
- SVG/raw HTML nincs;
- hibás vagy túl nagy dimenziójú kép `415` és rollback;
- handler/runtime hiba esetén a frissen feltöltött fájl törlődik;
- a media endpoint nem készít sessiont és nem futtat `.vrn` route-ot;
- a data root többi fájlja nem érhető el a media URL-en.

## M33 határ

Még nincs:

- resize/thumbnail pipeline;
- WebP/AVIF konverzió;
- EXIF-orientáció normalizálás vagy metadata stripping;
- media collection/admin böngésző;
- S3/object-storage backend;
- Markdown image syntax.

Ezeket érdemes külön milestone-okban hozzáadni, különösen a képdekódolást a `compute` resource profile alá kötve.
