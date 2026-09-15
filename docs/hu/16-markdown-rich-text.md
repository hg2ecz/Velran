# Markdown és rich text

A Markdown renderer **nem a Velran engine része**. A referencia-megvalósítás Velran nyelven írt forrásmodul: `common/commonmark.vrn`. Egy alkalmazásba forrásszinten emelhető be.

```velran
mod commonmark;

#[page]
fn article(ctx: PageContext, db: Db, id: i64) -> Result<Html, PageError> {
    let article = articleById(db, id)?;
    let markdown_html = commonmark::render(&article.body);
    return Ok(html {
        <article class="article-body">
            {{ markdown_html }}
        </article>
    });
}
```

A `commonmark::render` visszatérési típusa `SafeHtml`. Ez nem tetszőleges raw HTML string: csak a Velran általános, típusos HTML-biztonsági primitívjei állíthatják elő. Normál `String` interpoláció továbbra is escape-elt.

## Engine és modul határa

Az engine-ben nincs Markdown parser és nincs `@markdown(...)` direktíva. Az engine kizárólag általános primitíveket biztosít: escaped text, engedélyezett statikus HTML elem, biztonságos link, fragment-összefűzés és az opaque `SafeHtml` típus.

A Markdown szintaxis értelmezése teljes egészében a `commonmark.vrn` feladata. Emiatt a renderer auditálható, cserélhető és továbbfejleszthető Velran-kódként, compiler/runtime módosítás nélkül.

## A CommonMark mint nyelvi kompatibilitási példa

A renderer nem csak Markdown-példa: a valódi Velran library-programozás fontos kompatibilitási tesztje. Használ by-value `i64`/`bool` pure paramétereket, helper-hívásokat expression argumentumokkal, `if`/`else if`/`else` kontrollfolyamot, explicit immutable string borrow-t, `.to_string()` műveletet és depth-bounded scalar rekurziót. Ezeket a compiler általános nyelvi feature-ként biztosítja, nem CommonMark-specifikus kerülőúttal.

Részletes pure contract: [Verifikált pure függvények](58-verifikalt-pure-fuggvenyek.md).

## Biztonsági modell

- nincs raw HTML pass-through;
- `<script>`, `<iframe>` és más HTML markup escape-elt szövegként jelenik meg;
- nincs `unsafeHtml`/`raw` konstruktor;
- a tagokat az engine fix SafeHtml allowlistje korlátozza;
- link csak relatív vagy fragment célra, illetve `https`, `http` és `mailto` sémára készül;
- veszélyes link (`javascript:`, `data:`, `file:` stb.) nem válik linkké;
- a sorfeldolgozás `splitBounded(..., 2048)` korláttal működik.

A `commonmark` modul explicit block- és inline-parserrel kezeli az alkalmazásokban fontos CommonMark magot: ATX/Setext címsorok, thematic break, behúzott és változó hosszúságú fenced code blockok, bounded nested blockquote, csoportosított listák, több soros bekezdések, soft/hard line break, tetszőleges hosszúságú backtick code span, strong/emphasis, inline link, URI/e-mail autolink és punctuation-only backslash escape. A CRLF/CR sorvégek normalizálódnak. A CommonMark által egyébként megengedett raw HTML block/inline tudatosan tiltott; kép-szintaxis, reference-link definíciók, a teljes loose/nested-list szemantika, entity-dekódolás és ordered-list `start` attribútum még nincs leképezve. Ez a Velran security profile része.

## CMS ajánlás

Az adatbázisban az eredeti Markdown maradjon a source of truth. Kiszolgáláskor `commonmark::render(...)` készítse a `SafeHtml` értéket. Publikus, nem személyre szabott oldalnál a kész HTTP response opcionálisan route cache-be tehető.

Általános SQL + opcionális public response cache példa: [`examples/markdown-sql-cache/`](../../examples/markdown-sql-cache/README_hu.md). Az önálló source-module példa: [`examples/commonmark/`](../../examples/commonmark/README_hu.md).
