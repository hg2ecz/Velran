<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Atomikus session rotáció

A Velranban a session rotáció credential-életciklus állapotátmenet, nem közönséges update query.
A nyelv egyszerre követeli meg a régi, bemutatott token bizonyítékát és az új, frissen kiadott token bizonyítékát.

```vrn
model Session {
    id: i64
    tokenHash: Secret<SessionTokenHash>
    expiresAt: DateTime
}

#[query]
fn rotateSession(
    tx: Transaction,
    tokenHash: SessionTokenHash,
    newTokenHash: Secret<SessionTokenHash>,
    expiresAt: DateTime
) -> Result<Changed, DbError>
    rotates Session by tokenHash to newTokenHash until expiresAt
sql {
    UPDATE sessions
    SET token_hash = :newTokenHash,
        expires_at = :expiresAt
    WHERE token_hash = :tokenHash
      AND expires_at > CURRENT_TIMESTAMP
}
```

A hívási oldalon a régi hash csak validált, bemutatott bearer tokenből származhat:

```vrn
let oldHash = presentedTokenHash(token);
```

Az új hash pedig kizárólag frissen kiadott tokenből:

```vrn
let newToken = newSessionToken();
let newHash = tokenHash(newToken);
```

Az állapotátmenet ezután egy tranzakción belül atomikus:

```vrn
transaction db {
    let changed = rotateSession(tx, oldHash, newHash, expiresAt)?;
}
```

A compiler elutasítja a rotációt, ha:

- a query nem `UPDATE`;
- a visszatérés nem `Changed`;
- a régi hash nem hordoz presented-token proofot;
- az új hash nem hordoz friss issued-token-hash proofot;
- az SQL nem cseréli le egyszerre a token hasht és az expiry-t;
- a `WHERE` nem a régi token hashre illeszt;
- a régi expiry nincs `CURRENT_TIMESTAMP` ellen védve.

A `tokenHash(...)` maga is `IssuedToken` proofot követel. Tárolóból visszaolvasott `Secret<SessionToken>` nem nevezhető át friss replacement tokenné egyszerű újrahasheléssel.

## Miért külön lépés a cookie delivery?

A beépített Velran szerver már birtokolja az autentikációs cookie-t (`__Host-velran_session`). Egy második alkalmazásszintű session cookie két versengő session-authorityt hozna létre. A biztonságos böngészős deliveryt ezért a meglévő auth/session alrendszerrel kell integrálni, nem párhuzamos cookie-modellt bevezetni.
