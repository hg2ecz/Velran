<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Repository teszt-erőforrások

Ez a könyvtár a repository-szintű verifikáció olyan erőforrásait tartalmazza, amelyek nem felhasználói oktatópéldák.

- `fixtures/security/`: security-negatív források, amelyeket a fordítónak el kell utasítania.
- `fixtures/negative/`: egyéb nyelvi/keretrendszer elutasítási fixture-ök.
- `manifests/example-entrypoints.txt`: minden Velran-forrást tartalmazó oktatópélda-könyvtár egy kanonikus belépési pontja.
- `manifests/native-pending.txt`: a native-status ellenőrzések által használt generált kompatibilitási/státusz lista.
- `NATIVE_STATUS.md`: verifikációs célú native lefedettségi jegyzetek.

A szétválasztás szándékos: az `examples/` alatt tanulásra alkalmas kód marad; a `tests/` alatt az elsődlegesen ellenőrzési célú kód és metaadat található.