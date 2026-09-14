<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Idempotens kritikus műveletek

A Velran a kritikus, állapotmódosító végpontoknál úgy tud replay-védelmet kikényszeríteni, hogy a fejlesztőnek nem kell Redis-lockolást vagy saját idempotency infrastruktúrát írnia.

```vrn
critical Payment {
    transaction
    audit
    idempotency
}

#[action]
fn charge(ctx: ActionContext, db: Db) -> Result<Json, PageError> critical Payment {
    transaction db {
        // állapotváltozás + audit
    }
    return Ok(json(true));
}

route charge POST "/charge"
    auth critical Payment
    idempotent
    => charge;
```

A kliens `Idempotency-Key` headert küld. A kulcs 16..128 karakteres, és ASCII betűt, számot, `-`, `_`, `.`, `:` karaktereket tartalmazhat.

A platform a kulcsot domain + route + autentikált principal szerint scope-olja, és fingerprinteli a HTTP methodot, targetet, content type-ot és body-t. Az atomikus claim Redis `SET NX` művelettel történik. A kész, nem 5xx válasz 24 óráig eltárolódik, és azonos kulcs + azonos kérés esetén újrafuttatás nélkül visszajátszható. Ugyanaz a kulcs eltérő kéréshez nem használható.

Az idempotens route Redis-t igényel. Szándékosan nincs memory fallback production semanticsként, mert több processz és restart mellett az nem adna erős replay-garanciát.

Ha a végrehajtás 5xx állapotban fejeződik be, a claim pending marad. Ez fail-closed viselkedés: a platform ilyenkor nem tudja bizonyítani, hogy a hiba előtt semmilyen side effect nem történt.
