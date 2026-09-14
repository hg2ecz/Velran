<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Lejárat-tudatos tokenellenőrzés

A session és password-reset credential időben korlátozott életciklusú. Ezért a Velran ezeknél a token purpose-oknál nem engedi a puszta egyezőségvizsgálatot.

Használd a `tokenActive(storedHash, presentedToken, expiresAt)` primitívet:

```vrn
model Session {
    id: i64
    tokenHash: Secret<SessionTokenHash>
    expiresAt: DateTime
}

let valid = tokenActive(session.tokenHash, presentedSessionToken, session.expiresAt);
```

A compiler megköveteli:

1. `Secret<SessionTokenHash>` + `SessionToken`, vagy `Secret<PasswordResetTokenHash>` + `PasswordResetToken` párost;
2. azonos purpose-ú, validált bemutatott tokent;
3. explicit `DateTime` lejárati értéket.

A `tokenMatches(...)` szándékosan csak CSRF token egyezőségre marad használható. Session vagy password-reset hash esetén compile-time security error (`SEC-A07-004`) keletkezik, így a fejlesztő nem tudja véletlenül kihagyni a lejárat ellenőrzését.

Runtime oldalon a `tokenActive(...)` SHA-256-tal hasheli a bemutatott bearer tokent, a fix méretű hasheket constant-time módon hasonlítja össze, és `false` eredményt ad, ha `expiresAt <= now`.

A primitív csak azt bizonyítja runtime, hogy a bearer érték egyezik a tartós hash-sel és még nem járt le. A visszaadott `bool` szándékosan nem authorization- vagy consume-once proof. A password-reset elfogyasztása és a session rotation/revocation külön lifecycle átmenet.
