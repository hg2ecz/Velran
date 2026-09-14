<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Velran webalkalmazás-fejlesztőknek -- LaTeX könyv

A magyar könyv fejezetenként `\input`-olt LaTeX forrásból épül, és ugyanazt a közös tipográfiai stílust használja, mint az angol kiadás.

A repository gyökeréből:

```bash
make book-hu
```

Vagy közvetlenül:

```bash
make -C docs/book/hu pdf
```

Kimenet:

```text
docs/book/hu/velran-webfejlesztoknek-hu.pdf
```

Tisztítás:

```bash
make -C docs/book/hu clean
make -C docs/book/hu distclean
```

A `distclean` a generált PDF-et is eltávolítja, így a forrás-release csomag tiszta marad.

## Szerkezet

A könyv első olvasásra, kezdő Velran-felhasználónak van rendezve:

1. modell-szemlélet, telepítés, A-Z minimális alkalmazás;
2. alap webalkalmazás-fejlesztés;
3. wiki/CMS gyakorlati alkalmazás;
4. haladó integráció és tesztelés;
5. production és üzemeltetés;
6. referencia és operátori gyorssegédlet.

A teljes `server.toml` referencia szándékosan a könyv végén van: az első alkalmazáshoz csak a szükséges minimális konfigurációt vezetjük be. A tanulói companion alkalmazások az `examples/` alatt vannak; a csak verifikációs rejection esetek a `tests/fixtures/`, a verification metaadatok pedig a `tests/manifests/` alatt. Az SQL-ből olvasott, szerveroldalon renderelt Markdown és opcionális public cache fókuszált példája az `examples/markdown-sql-cache/`.

### Secure Notes checkpoint-szabály

A folyamatos Secure Notes mintaprojekt gyökere az `examples/secure-notes/`. A fejezeti checkpointokat az `examples/secure-notes/CHECKPOINTS.tsv` térképezi fel; minden felsorolt `main.vrn` fájlt a `verify.sh` külön compiler-checkeli. A fókuszált framework-példák forrása továbbra is a `docs/book/CANONICAL_EXAMPLES.tsv` companion manifest.
