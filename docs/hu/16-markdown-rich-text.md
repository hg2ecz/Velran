<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Biztonságos Markdown és rich text

**Státusz: IMPLEMENTÁLT (M32).**

Velranban adatbázisból vagy requestből érkező szerkesztett szöveget ne interpolálj raw HTML-ként. A HTML template content pozíciójában használd a typed Markdown direktívát:

```velran
#[page]
fn article(ctx: PageContext, db: Db, id: i64) -> Result<Html, PageError> {
    let article = articleById(db, id)?;
    return Ok(html {
        <article class="article-body">
            @markdown(article.body)
        </article>
    });
}
```

Az `@markdown(...)` argumentuma kizárólag `String` lehet. Attribútumban nem használható.

## Biztonsági modell

A renderer nem fogad el raw HTML-t. A Markdown inputban található `<script>`, `<iframe>`, event handler vagy más HTML markup szövegként, HTML-escape-elve jelenik meg. Nincs `unsafeHtml`, `raw`, vagy sanitizer-bypass API.

Az M32 allowlistje:

- `#`–`######` heading;
- bekezdés;
- `**strong**`;
- `*emphasis*`;
- `` `inline code` ``;
- fenced code block három backtickkel;
- egyszerű unordered list (`- ` vagy `* `);
- Markdown link szintaxis támogatott (`[szöveg]` + zárójeles cél).

A link cél csak az alábbi lehet:

- `#anchor`;
- root-relative `/path`, de nem `//host/path`;
- `https://...`;
- `http://...`.

Más séma — például `javascript:`, `data:`, `file:` — nem kap aktív `<a href>` elemet.

A generált külső/belső link `rel="noopener noreferrer"` attribútumot kap.

## Erőforrás-korlátok

Egy Markdown érték legfeljebb 512 KiB lehet rendereléskor. A feldolgozás beleszámít a request instruction és runtime allocation budgetbe. Az inline markup egymásba ágyazási mélysége maximum 32; túllépés fail-closed runtime hibát ad.

Ez a limit renderelési védőkorlát, nem feltöltési vagy adatbázis mezőméret-policy.

## Mit nem támogat az M32

Szándékosan nincs még:

- raw HTML;
- image Markdown (`![...]()`);
- táblázat;
- footnote;
- embedded video/iframe;
- automatikus linkify;
- WYSIWYG editor;
- saját HTML attribútum Markdownból.

A képfeltöltés és média külön, typed media-library capabilityként érdemes, nem tetszőleges Markdown URL-ként.

## CMS ajánlás

Adatbázisban az eredeti Markdown szöveget tárold, ne a renderelt HTML-t. Rendereléskor mindig `@markdown(...)` készítse a HTML-t. Így a biztonsági policy egy helyen, a runtime-ban marad, és későbbi renderer-javítások a meglévő tartalomra is érvényesülnek.

Általános SQL + opcionális public HTML response cache példa: [`examples/markdown-sql-cache/`](../../examples/markdown-sql-cache/README_hu.md).
