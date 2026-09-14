<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Típusos szerverkonfigurációs és TLS hibák

A szerver a dinamikus hibatípus-eróziót csak a legkülső alkalmazáshatáron használja. A konfigurációt, hostnevet, statikus URL-prefixet, secret fájlt és TLS anyagokat ellenőrző belső leaf modulok típusos hibákkal térnek vissza.

## Miért

A konfigurációs és TLS hibák üzemeltetési szempontból eltérnek az alkalmazáskérések hibáitól. Ha minden helper `Box<dyn Error>` vagy egyszerű `String` hibát ad, ez a különbség elveszik, a tesztek pedig hibaüzenet-szövegre kényszerülnek támaszkodni.

A szerver most külön típusokat használ:

- `ServerConfigError` – server/domain config betöltés és validáció;
- `TlsConfigError` – cert/key betöltés és rustls konfiguráció;
- `PublicHostError` – canonical public host validáció;
- `StaticPrefixError` – statikus URL-prefix validáció;
- `SecretFileError` – nem olvasható vagy üres secret fájl.

Az I/O és TOML parse hibák megőrzik az eredeti source errort és a kapcsolódó fájlútvonalat. A validációs hibák stabil kategóriákat adnak, miközben a konzolon továbbra is emberileg jól olvasható hibaüzenet jelenik meg.

## Alkalmazáshatár

A `startup::run()` szándékosan aggregáló határ marad, mert adatbázis-, Redis-, auth-, storage-, resource-limit-, TLS- és config-crate hibákat kapcsol össze. Ezen a legfelső szinten elfogadható a heterogén hibák eróziója, de a leaf modulokban új `Box<dyn Error>` API-t már nem célszerű bevezetni.
