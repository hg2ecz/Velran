<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Platform által kezelt böngészős session

A Velran böngészős autentikációja egyetlen, platform által kezelt session authorityt használ. Az alkalmazáskódnak nem kell böngészős autentikációs cookie-t generálnia, rotálnia, serializálnia vagy konfigurálnia.

## Biztonságos happy path

A szerver garantálja:

- sikeres autentikáció után új session azonosító és új CSRF token keletkezik;
- logoutkor az autentikált session érvénytelen lesz, majd friss anonymous session jön létre;
- lokális jelszó-, MFA-, disabled-state- és más auth-generation változás után a régi autentikált session a következő kérésnél érvénytelen lesz;
- production cookie neve `__Host-velran_session`, továbbá `Path=/`, `HttpOnly` és `Secure` attribútumot kap;
- alapértelmezésben `SameSite=Lax`, credentialed cross-origin telepítésnél `SameSite=None`, de továbbra is `Secure`;
- nincs `Domain` attribútum, ezért a cookie host-only marad.

A normál Velran alkalmazáskód ezért autentikációs szándékot fejez ki (`auth user`, `auth mfa`, `auth role ...`), nem session transport mechanikát.

## Fejlesztői ergonomia

A security-critical cookie policy egyetlen szervermodulban él és regressziós tesztek védik. Nem kell route-onként cookie flag-eket vagy login utáni kézi rotation hívásokat írni.

Alkalmazásszintű bearer token továbbra is indokolt explicit domain workflow-khoz, például password resethez vagy API credentialhöz. Ezeket nem szabad második böngészős login-session rendszerként használni a platform session mellett.
