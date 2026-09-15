<!-- VELRAN-DOC-STATUS: 2026-09-15 -->
> **Dokumentációs státusz (2026-09-15):** A verified pure compute aktuális nyelvi szerződése. Első a biztonság; a produktivitás csak úgy bővül, hogy a compiler megőrzi a típus-, authority-, aliasing- és resource-invariánsokat.

# Verifikált pure függvények

A verified pure függvény a Velran újrafelhasználható alkalmazás-compute rétege. Rust-szerű `fn`, amely ambient web/server authority nélkül fut, verifikált IR-en keresztül generált safe Rust kódra fordul.

Parser, renderer, validációs helper, transzformáció, numerikus kernel és más olyan számítás való ide, amelynek nincs szüksége közvetlen adatbázis-, fájlrendszer-, hálózat-, process-, environment-, thread-, FFI- vagy `unsafe` hozzáférésre.

## Paramétercsaládok

A scalar/borrowolt család jelenleg:

```text
i64
bool
&str
&[String]
&Struct
```

Az `i64` és `bool` immutable, by-value paraméter. A string, string-lista és struct borrow explicit marad.

A mutable numerikus hot-path család fix tömböt támogat:

```text
&mut [f32; N]
```

A mutable tömböt a hívási ponton is explicit `&mut` formában kell átadni:

```velran
kernel(&mut real)
```

A compiler nem alakítja át automatikusan a `kernel(real)` formát mutable borrow-vá. A scalar/borrowolt és mutable numerikus paramétercsalád egy függvényben jelenleg nem keverhető, mert külön verifikált belső ABI-t használ.

## Visszatérési értékek

A verified pure függvények a jelenleg verifikált biztonságos return surface-et használhatják: scalar értékek, owned `String`/string-lista, `SafeHtml`, deklarált structok és a támogatott `Option<T>` / `Result<T,E>` formák.

A nested `if`/`else`/`else if`/`while` blokkok `return` ágai típusellenőrzöttek. Nem-unit függvénynél a jelenlegi lowering továbbra is megkövetel egy végső top-level returnt; a nested early return ezt nem szünteti meg.

## Pure → pure hívások

Scalar/borrowolt pure függvény hívhat más scalar/borrowolt pure helpert. Az argumentum normál típusos expression lehet:

```velran
fn step(value: i64, enabled: bool) -> i64 {
    if enabled {
        return value + 1;
    }
    return value;
}

fn run(value: i64) -> i64 {
    let first = step(value + 1, true);
    let mut out = 0;
    out = step(first, false);
    return out;
}
```

A child hívás ugyanabban az instruction/allocation budget-elszámolásban marad. A scalar és mutable numeric helper család nem hívhat át egymás belső ABI-jára; a scalar orchestration és mutable numeric kernel maradjon külön.

## Rekurzió és resource safety

A scalar/borrowolt pure rekurzió támogatott, de runtime depth-bounded. A generált kód aktuális hard maximuma 64 nested pure hívás. Az instruction- és allocation-budget ettől függetlenül továbbra is érvényes.

A mutable numerikus hot-pathot érintő rekurzív ciklus compile-time hiba. Ez fail-closed módon védi a speciális mutable-array ABI-t a kontrollálatlan rekurzív stack növekedéstől.

## Kontrollfolyam

Normál Rust-szerű elágazás használható:

```velran
if condition {
    ...
} else if other_condition {
    ...
} else {
    ...
}
```

Minden feltétel pontosan egyszer értékelődik ki. A compiler a belső temporálisokon is megőrzi a feltétel statikus security metadata-ját; untrusted boolból nem lesz véletlenül trusted bool.

A `while` továbbra is resource-budgetelt, és egyik kontrollfolyam-konstrukció sem ad új capabilityt a pure függvénynek.

## Stringek és immutable borrow

A Rust-szerű string metódusok — például `.trim()`, `.to_lowercase()`, `.to_string()` — verifikált stringműveletek. A Stringet előállító műveletek allocation-budgetbe számítanak.

Stringet váró builtin argumentumhelyen explicit immutable borrow is használható:

```velran
let tail = substring(&text, 1);
let first = charAt(&tail, 0);
```

A compiler nem törli általánosan az `&` jelet. Ahol a builtin contract nem fogad immutable borrow-t, ott compile error marad. `&mut` nem helyettesítheti az immutable string borrow-t.

## Template és SafeHtml határ

A `SafeHtml` valódi típusos output boundary, nem „megbízható string” konvenció. Normál `String` HTML-template interpolációnál escape-elődik; `SafeHtml` nem kap újabb escapinget.

A compiler/framework csak az általános SafeHtml primitíveket, escapinget, tag allowlistet és URL-policyt birtokolja. A domain-specifikus renderelő logika alkalmazás/library feladat. Ezért a Markdown/CommonMark Velran forrásmodul (`common/commonmark.vrn`), nem engine feature.

Nincs `@markdown(...)` engine direktíva. Az ismeretlen `@name(...)` alakú template call-direktíva fail-fast compile error, nem csendben literal szöveg; ez az elgépeléseket is azonnal megfogja.

## Diagnosztika

A frontend expression- és kontrollfolyam-hibák megőrzik a forráspozíciót. Fájlból fordítva a diagnosztika fájlnevet és sorszámot ad; a nested `if`/`else`/`while` parser sem veszíti el az eredeti line base-t.

A diagnosztikának a valódi megsértett nyelvi contractot kell megneveznie. Legacy szintaxis például legacy syntax hibát ad, nem félrevezető borrow hibát.

## Biztonsági invariánsok

A produktivitási bővítések nem gyengíthetik ezeket:

- nincs ambient filesystem/network/process/environment/thread/FFI/unsafe authority;
- a mutable borrow explicit;
- a scalar/borrowolt és mutable numeric helper ABI külön marad;
- a pure hívások öröklik és terhelik a resource budgetet;
- scalar rekurzió hard depth-bounded;
- numeric-hot-path rekurzív ciklus fail-closed;
- nested kontrollfolyamban is típusellenőrzött expression és return;
- string allokáció budgetelt;
- a `SafeHtml` típusos XSS boundary marad.

Tervezési szabály: **első a biztonság, második a produktivitás — de normális nyelvi konstrukciót ne kényszerítsünk mesterséges átírásra, ha az invariánsok gyengítése nélkül implementálható.**
