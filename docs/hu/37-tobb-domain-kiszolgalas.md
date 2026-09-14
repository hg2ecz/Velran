<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 37. Több domain kiszolgálása

A Velran vagy a régi, egyalkalmazásos `server.app` módot, vagy a `[[domains]]` alapú többdomaines módot használja. A kettő egyszerre nem állítható be.

Minden domainnek van egy elsődleges `host` neve, és tetszőleges számú `aliases` neve lehet. Az elsődleges név és minden alias ugyanarra a lefordított alkalmazásra, munkakönyvtárra, storage/static gyökérre, resource profile készletre és domainenkénti request budgetre mutat. Ugyanaz a hostname vagy alias két domainhez nem rendelhető. Ismeretlen `Host` esetén a szerver `421 Misdirected Request` választ ad.

```toml
[[domains]]
host = "example.com"
aliases = ["www.example.com", "example.net", "www.example.net"]
workdir = "/srv/velran/domains/example.com/current"
app = "main.vrn"
```

A domain `workdir` abszolút útvonal. Az alkalmazás, storage, static asset és domain resource-profile relatív útvonalai ezen belül oldódnak fel, és `..` használatával sem léphetnek ki belőle.

A globális `[limits]` marad a közös alap. A process/listener szintű limitek — például `max_connections`, `max_header_bytes`, `max_process_memory_bytes` és `[cgroup]` — globális hard policyk. Domainenként csak a request/runtime budgetek írhatók felül: `max_body_bytes`, `request_timeout_ms`, form limitek, `max_instructions`, `max_runtime_alloc_bytes`, `max_concurrent_requests` és `resource_profiles_file`.

A domain külön `config_file` fájlból is includolható. Az inline értékek felülírják az include értékeit. Az `aliases` lista egyetlen beállításnak számít: ha inline meg van adva, az inline lista lecseréli az include-ban lévőt.

## TLS tanúsítványok és SNI

Közvetlen többdomaines TLS esetén a szerver SNI alapján választ tanúsítványt. Egy domain saját cert/key párt kaphat:

```toml
[domains.tls]
cert_file = "/run/secrets/velran/example.com-fullchain.pem"
key_file = "/run/secrets/velran/example.com-key.pem"
```

A tanúsítványnak le kell fednie az elsődleges `host` nevet és az összes aliast. A Velran ezeket induláskor regisztrálja a rustls SNI resolverben; ha a cert valamelyik konfigurált névre nem érvényes, a startup/preflight hibával leáll.

A globális `[tls] cert_file/key_file` fallbackként szolgál azoknak a domaineknek, amelyekhez nincs `[domains.tls]`. Így egy közös SAN vagy wildcard cert több domainhez is használható. A domainszintű cert/key felülírja a globálisat az adott domainre és minden aliasára. Ha a direct TLS aktív, de valamely domainhez sem saját cert, sem globális fallback nincs, a szerver nem indul el.

TLS módban a HTTP `Host` értékét a TLS SNI hostname-hez kötjük. Így az egyik névre hitelesített TLS kapcsolat nem használható egy másik konfigurált hostname címzésére. Az Origin/Referer ellenőrzés az aktuálisan használt aliasból indul ki, ezért az aliasok state-changing browser kéréseknél is helyesen működnek.

A cert és key útvonalak abszolút, operátori kezelésű útvonalak, nem a domain workdir alatt oldódnak fel. A jelenlegi szerver induláskor tölti be a tanúsítványokat, ezért megújítás után a service managertől restart/reload szükséges.

A beépített egyhostos HTTP redirect listener többdomaines módban továbbra sem használható; a HTTP→HTTPS átirányítást reverse proxyban végezd.


Az újonnan feltöltött `.vrn` kód automatikus, dependency-gráf alapú és tranzakciós érvényesítését lásd az [Automatikus alkalmazás-forráskód reload](38-automatikus-forraskod-reload.md) fejezetben.
