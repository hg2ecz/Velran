<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Domain objectek nagyobb alkalmazásokhoz

Kisebb Velran alkalmazásban teljesen rendben van a top-level modell + query + page/action felépítés. Nagyobb CMS, hírportál vagy ügyviteli rendszer esetén viszont gyorsan sok globális név keletkezik.

Az ajánlott megoldás a `mod` fájlszervezés és az `object` domain-szervezés együtt.

## Alapelv

```text
mod     = melyik fájlban van a kód
object  = melyik üzleti fogalomhoz tartozik
route   = milyen HTTP felületen érhető el
```

Az `object` nem klasszikus OOP-osztály. Inkább egy domain namespace, amelyhez egy typed model és a hozzá tartozó műveletek tartoznak.

## Első példa

```velran
object Article {
    model {
        id: i64
        slug: Slug
        title: String
        body: String
        authorUsername: String
    }

    #[query]
fn bySlug(db: Db, slug: Slug) -> Result<Article, DbError> sql {
        SELECT id, slug, title, body, authorUsername
        FROM articles
        WHERE slug = :slug
    }

    #[page]
fn show(ctx: PageContext, db: Db, slug: Slug) -> Result<Html, PageError> {
        let article = Article.bySlug(db, slug)?;
        return Ok(html {
            <article>
                <h1>{{ article.title }}</h1>
                @markdown(article.body)
            </article>
        });
    }
}

route articleShow GET "/cikk/:slug<Slug>" public => Article.show;
```

A fejlesztő így `Article.bySlug`, `Article.show`, később például `Invoice.approve` vagy `User.disable` nevekkel dolgozik, nem több száz globális `articleBySlug`, `invoiceApprove`, `userDisable` függvénnyel.

## Mit tartalmazhat egy object?

M37-ben:

```text
model { ... }
#[query]
fn ...
#[page]
fn ...
#[action]
fn ...
#[component]
fn ...
#[layout]
fn ...
```

Pontosan egy `model` blokk kötelező.

A route szándékosan top-level marad. Például:

```velran
route articleShow GET "/cikk/:slug<Slug>"
    public cache public ttl 60
    => Article.show;

route articlePublish POST "/admin/cikk/:id<i64>/publish"
    auth role Publisher
    invalidate cache articleShow
    => Article.publish;
```

Így code review során az URL, auth, rate limit, cache és invalidation policy továbbra is egy helyen áttekinthető.

## Capabilityk nem lesznek mágikusak

Egy object nem kap automatikusan adatbázist vagy current usert.

Jó:

```velran
#[query]
fn byId(db: Db, id: i64) -> Result<Article, DbError> sql { ... }

#[action]
fn publish(ctx: ActionContext, db: Db, id: i64)
    -> Result<Redirect, PageError>
{
    let article = Article.byId(db, id)?;
    authorize article owner authorUsername or role Publisher;
    ...
}
```

Nem létezik olyan implicit forma, amelyből az object magától szerez DB/session/filesystem/network hozzáférést.

## Modulokkal együtt

Ajánlott nagyobb projekt:

```text
main.vrn
article.vrn
article/
  admin.vrn
invoice.vrn
account.vrn
routes.vrn
```

`main.vrn`:

```velran
mod article;
mod invoice;
mod account;
mod routes;
```

`article.vrn` tartalmazhatja az `Article` objectet. Ha az `article/admin.vrn` is része az alkalmazásnak, azt például `mod article::admin;` deklaráció tölti be. Más modulból az object neve `article::Article`, a member pedig például `article::Article.bySlug`.

## Névkonvenció

Object: üzleti főnév, PascalCase:

```text
Article
Invoice
Customer
UserAccount
```

Member: rövid, domainen belül értelmes camelCase:

```text
Article.bySlug
Article.publish
Invoice.approve
Invoice.cancel
```

A route neve viszont maradjon auditbarát, globálisan beszédes:

```text
articleShow
articlePublish
invoiceApprove
```

Ez azért fontos, mert az access/audit logban a route neve jelenik meg.

## Amit nem csinál az object

Nincs:

- inheritance / `extends`;
- mutable `self`;
- constructor-mágia;
- virtual method;
- runtime dispatch;
- reflection;
- hidden dependency injection.

A Velran ezzel domain-oriented nyelv lesz, nem általános célú klasszikus OOP nyelv.

## Enum következő lépés

Üzleti rendszernél az olyan állapotok, mint `Draft`, `Review`, `Published`, `Paid`, `Cancelled` first-class enumot igényelnek. Ezt nem Stringként álcázza az M37; külön típus lesz, amely a teljes request/DB/runtime stacken ellenőrzött.
