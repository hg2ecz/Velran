<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Purpose-biztos token primitívek

A Velran a protokoll-tokeneket purpose-specifikus primitívekkel állítja elő, nem általános random String API-val:

```vrn
let session = newSessionToken();
let reset = newPasswordResetToken();
let csrf = newCsrfToken();
```

Az eredmények statikusan secret credentialök:

- `newSessionToken()` -> `Secret<SessionToken>`
- `newPasswordResetToken()` -> `Secret<PasswordResetToken>`
- `newCsrfToken()` -> `Secret<CsrfToken>`

A runtime 256 bit kriptográfiailag biztonságos véletlent generál és 64 kisbetűs hex karakterként reprezentálja. Az alkalmazáskód nem választhat gyengébb RNG-t vagy rövidebb tokenhosszt.

## Ellenőrzés

A `tokenMatches(storedHash, presented)` csak akkor érvényes, ha:

1. a tárolt érték secret token credential;
2. a bemutatott token pontosan ugyanahhoz a purpose-höz tartozik;
3. a bemutatott token validáló request boundaryn érkezett.

Ebben a rétegben a támogatott purpose-ök: `SessionToken`, `PasswordResetToken`, `CsrfToken`. A reset token nem használható session tokenként csak azért, mert runtime mindkettő String reprezentációjú.

A runtime azonos hosszúságú tokenek tartalmát constant-time primitívvel hasonlítja össze. A session/reset/CSRF request tokenek a fix 64 karakteres generált reprezentációt követik.

## Nincs generikus delivery escape hatch

A generált tokenek továbbra sem renderelhetők generikus HTML/JSON válaszba. Explicit `Secret<TokenPurpose>` persistence sinkbe mehetnek, de cookie-, reset-link- és CSRF-form delivery külön protokoll-boundary feladata lesz.

A `tokenMatches` a bemutatott tokent a tartós hash ellenőrzi; nem ad authorization proofot, és nem bizonyít lejáratot vagy single-use felhasználást. Ezek a következő lifecycle-réteg feladatai.
