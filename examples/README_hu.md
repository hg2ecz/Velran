<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Velran példák

Ez a könyvtár a felhasználóknak szánt példákat tartalmazza: oktatóanyagokat, funkcióbemutatókat, induló alkalmazásokat, deployment/konfigurációs mintákat és olyan teljesítménypéldákat, amelyek olvasásra vagy továbbépítésre is hasznosak.

A verifikációs rendszer ahol lehet, ezeket a példákat is lefordítja, de ettől még nem teszt-fixture-ök. A kanonikus példa-belépési pontok listája a `tests/manifests/example-entrypoints.txt` fájlban található.

A fordítónak szándékosan elutasítandó és security-negatív esetek a `tests/fixtures/` alatt vannak. A verifikációs metaadatok a `tests/manifests/` alatt találhatók.

Egy példa akkor is itt maradhat, ha a `verify.sh` ellenőrzi, amennyiben önálló oktatási vagy dokumentációs értéke van.