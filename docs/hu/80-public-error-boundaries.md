<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Publikus hibahatárok

A Velran különválasztja az alkalmazás által biztonságosan publikálható hibákat a belső runtime hibáktól.

Egy page vagy action az alábbi zárt hibakészlettel állhat le:

```vrn
fail badRequest;
fail notFound;
fail forbidden;
fail conflict;
```

A szerver ezeket a meglévő biztonságos HTTP hibaválaszokra képezi. Alkalmazáskódból nincs `fail internal`, `fail database`, tetszőleges HTTP státuszkód vagy tetszőleges hibaszöveg. A belső, adatbázis-, erőforrás-limit- és infrastruktúrahibák platform-owned hibahatáron maradnak.

Így a normál fejlesztői út rövid, miközben stack trace, SQL részlet, credential vagy infrastruktúraüzenet nem tud véletlenül HTTP válaszba kerülni.

A `fail` terminális statement, ugyanúgy lezárja a page/action bodyt, mint egy sikeres return.

A `SEC-A10-003` diagnosztika tiltja az ismeretlen vagy belső hibatípus publikálását.
