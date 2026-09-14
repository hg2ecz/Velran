<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Típusos auth-, policy-, resource-profile- és CLI-hibák

Az R10 továbbviszi a szerver clean-code refaktorját: a megmaradt konfigurációs és parancssori leaf API-k is típusos hibákat adnak.

## Alapszabály

A leaf modulok megőrzik a hiba kategóriáját és ahol lehet, az eredeti `source` hibát is. A process-szintű `startup::run()` továbbra is használhat `Box<dyn Error>` aggregációs határt, mert ott sok külön crate hibája találkozik.

## Auth setup

Az `auth_setup.rs` már `AuthSetupError` típussal tér vissza. Külön variáns van a hiányzó LDAP mezőkre, LDAP validációra, auth-fájl I/O hibára, hibás TOTP/role sorra, hibás vagy duplikált felhasználónévre, hibás hex secretre, túl rövid TOTP secretre és hibás role névre.

Így a startup-hibák teszteléséhez nem kell hibaüzenet-szövegeket összehasonlítani.

## Rate-policy konfiguráció

A `load_rate_policies` és `validate_route_rate_policies` `RatePolicyConfigError` típust ad. Elkülönül az I/O, a sor-szintű szintaktikai hiba, a hiányzó mező, hibás szám, hibás limit, ismeretlen scope/key, ismeretlen route policy és a publikus route-on tiltott user-scoped policy.

## Resource profile

A `load_resource_profiles` és `audit_resource_profiles` `ResourceProfileConfigError` típust használ. A fájl és sorszám megmarad, a számkonverziós és runtime `ResourceProfileError` okok pedig nem lapulnak egyszerű stringgé.

## CLI parsing

A `cli::parse_args()` most `CliParseError` típussal tér vissza `Box<dyn Error>` helyett. A config, secret-file, host/static-prefix, reserved path, numerikus CLI érték, policy/profile, compiler, TLS, I/O, cím-parse és integer-konverziós hibák külön kategóriák.

A kis numerikus helper-ek `CliValueError` típust adnak: hiányzó érték, hibás szám és nullánál nagyobb érték követelménye külön variáns.

## Error-modul felosztás

Az új hibák nem egy új god-file-ba kerültek:

- `server_errors.rs` — közös server-config/TLS hibák és re-exportok;
- `server_errors/auth_setup.rs` — auth bootstrap;
- `server_errors/policy_config.rs` — rate-policy és resource-profile;
- `server_errors/cli.rs` — CLI és reserved endpoint hibák.

A clean-structure guard tiltja, hogy ezek a leaf API-k később visszacsússzanak boxed hibákra.
