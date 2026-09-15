# Forrásszintű CommonMark renderer

A `commonmark.vrn` egy Velran nyelven megírt, CommonMark-orientált Markdown renderer. Nem a compiler/runtime engine része: nincs Markdown-specifikus engine feature.

Másold a `commonmark.vrn` fájlt az alkalmazás `main.vrn` fájlja mellé, majd emeld be:

```vrn
mod commonmark;
```

Renderelés:

```vrn
let rendered = commonmark::render(&source);
```

A `SafeHtml` érték közvetlenül interpolálható a `html { ... }` template-ben. A normál `String` továbbra is HTML-escape-elve kerül kiírásra.

## Velran nyelvi szerep

Ez a példa általános Velran nyelvi stresszteszt is, nem framework-kivétel. A renderer verified pure helper kompozíciót, by-value scalar paramétereket, explicit immutable borrow-t, Rust-szerű `if`/`else if`/`else` kontrollfolyamot, bounded scalar rekurziót és allocation-budgetelt stringeket használ. Ezek egyike sem Markdown-specifikus nyelvi privilégium.

Az ismeretlen `@name(...)` alakú template call-direktíva compile error. Nincs Markdown-specifikus engine direktíva; az engine-oldali privilégium kizárólag az általános, típusos `SafeHtml` boundary.

## Biztonsági profil

A modul tudatosan biztonságos CommonMark-profilt használ:

- a Markdownban lévő raw HTML szövegként jelenik meg, nem kerül pass-through módon a válaszba;
- minden normál szöveg a `safeHtmlText` primitíven megy át;
- a kimeneti elemeket az engine kis, rögzített `SafeHtml` tag-allowlistje korlátozza;
- linknél csak relatív/fragment, `https`, `http` és `mailto` cél engedélyezett;
- a feldolgozás legfeljebb 2048 sorra bounded;
- nincs általános raw-HTML konstruktor az alkalmazás számára.

A forrásmodul explicit block- és inline-parserrel kezeli az alkalmazásokban fontos CommonMark magot: ATX és Setext címsorok, thematic break, behúzott és változó hosszúságú fenced code blockok, bounded egymásba ágyazott blockquote-ok, csoportosított rendezetlen/rendezett listák, több soros bekezdések, soft és hard line break, tetszőleges hosszúságú backtick code span, strong/emphasis, kiegyensúlyozott zárójeles inline link, URI/e-mail autolink és csak írásjelekre érvényes backslash escape. A block parsing előtt a CRLF/CR sorvégek normalizálódnak.

A raw HTML block/inline szándékosan tiltott. Kép-szintaxis, reference-link definíciók, a teljes loose/nested-list szemantika, entity-dekódolás és a rendezett listák `start` attribútuma még nincs kimenetre képezve. Ez ezért security-hardened CommonMark profil, nem minden upstream renderelési szabály byte-for-byte implementációja.
