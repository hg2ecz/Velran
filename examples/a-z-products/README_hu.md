<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# A-Z mini termeklista

A konyv teljes, elejetol vegeig vezetett minimalis peldaja.

1. Keszits egy secret fajlt, amely egy abszolut SQLite URL-t tartalmaz, peldaul:
   `sqlite:///tmp/velran-a-z-products.db`
2. Ellenorizd a forrast: `velran-cli check main.vrn`.
3. Alkalmazd a migraciot:
   `velran-cli migrate apply --dir migrations --db-url-file dev-db-url`.
4. A `server.toml.example` abszolut pathjait igazitsd a gepedhez.
5. Inditsd: `velran-server --config server.toml`.
6. Nyisd meg: `http://127.0.0.1:8080/`.

A dev DB URL fajlt ne commitold valodi credentiallel.
