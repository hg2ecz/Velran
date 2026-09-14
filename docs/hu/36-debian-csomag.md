<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Debian csomag készítése és telepítése `dpkg -i` paranccsal

A Velran két telepítési konvenciót támogat. A kézi/forrásból történő telepítés a projektben használt lokális adminisztrátori prefixet követi:

```text
/usr/local/bin/velran-server
/usr/local/bin/velran-cli
/usr/local/etc/velran/server.toml
```

A Debian csomag más: a `dpkg` által birtokolt fájlok Debian-konvenció szerint kerülnek a rendszerbe. A `.deb` ezért `/usr/bin` alá telepíti a binárisokat és `/etc/velran` alá a konfigurációt; csomagkezelt fájlt nem tesz `/usr/local` alá.

## Csomag készítése

Debian/Ubuntu build gépen, Rust és `dpkg-deb` mellett:

```bash
make deb
```

vagy közvetlenül:

```bash
tools/package-deb.sh
```

A script locked release buildet készít a két publikus binárisból, majd alapértelmezés szerint létrehozza:

```text
dist/velran_1.0.0-1_<arch>.deb
```

Hasznos felülírások:

```bash
tools/package-deb.sh --version 1.0.0-2
tools/package-deb.sh --output-dir /tmp/packages
```

CI-ban már elkészített binárisok is csomagolhatók:

```bash
tools/package-deb.sh --skip-build --bin-dir target/release
```

## A csomag tartalma

```text
/usr/bin/velran-server
/usr/bin/velran-cli
/etc/velran/server.toml
/etc/velran/rate-limits.toml
/etc/velran/resource-profiles.toml
/usr/lib/systemd/system/velran.service
/usr/lib/tmpfiles.d/velran.conf
/etc/logrotate.d/velran
/usr/share/doc/velran/
```

Az `/etc/velran/server.toml`, a rate-limit/resource-profile policy fájlok és az `/etc/logrotate.d/velran` conffile-ok, ezért a helyi operátori módosításokat a `dpkg` frissítéskor a szokásos Debian módon kezeli.

A csomag létrehozza az `velran` rendszerfelhasználót/csoportot, valamint a `/srv/velran/data`, `/var/log/velran` és `/run/secrets/velran` könyvtárakat. A szolgáltatást szándékosan **nem indítja el automatikusan**, mert egy általános runtime csomag nem ismerheti az alkalmazást, a credentialöket, TLS kulcsokat, adatbázis URL-t és public hostot.

## Telepítés

```bash
sudo dpkg -i dist/velran_1.0.0-1_amd64.deb
```

Ezután telepítsd az alkalmazást és a secreteket, állítsd be az `/etc/velran/server.toml` fájlt, majd ellenőrizd:

```bash
sudo velran-server --config /etc/velran/server.toml --check-config
```

Csak sikeres ellenőrzés után indítsd:

```bash
sudo systemctl enable --now velran.service
```

## Közvetlen TLS és reverse proxy

A csomagolt konfigurációs minta a repository direct-TLS mintáját követi 80/443 porton. Ezért a systemd unit kizárólag `CAP_NET_BIND_SERVICE` capabilityt kap, így az unprivileged `velran` user is tud privileged portra bindolni.

Apache/Nginx mögött, például `127.0.0.1:8080` listenerrel ezt a capabilityt site-specific systemd override-ban távolítsd el. A TLS authority ilyenkor a reverse proxy, a Velran oldalon pedig explicit `public_host` és trusted proxy CIDR szükséges.
