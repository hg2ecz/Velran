<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 8. HTTPS és böngészőbiztonság

Production minimumot trusted configban adj meg:

```toml
[tls]
cert_file = "/run/secrets/tls/fullchain.pem"
key_file = "/run/secrets/tls/privkey.pem"
public_host = "app.example.com"
http_redirect_listen = "0.0.0.0:80"
```

A pontos kulcsokat a `config/server.toml.sample` mutatja; productionban a TOML az elsődleges interface, a CLI csak célzott override. A HTTP→HTTPS redirect a trusted public hostból készül, és csak GET/HEAD redirectelhető.

## CSRF

POST defense-in-depth:

```text
session-bound CSRF
+ same-origin Origin/Referer
+ Sec-Fetch-Site cross-site deny
```

Normál formban:

```html
<input type="hidden" name="_csrf" value="{{ csrfToken }}">
```

## Proxy

Trusted reverse proxy CIDR-t productionban szintén a server configban állíts be. A CLI megfelelője csak célzott override/development célra szolgál. Ha a publikus TLS-t Apache/Nginx terminálja, a Velran production upstream listener legyen loopback, maradjon `insecure_dev_cookies = false`, és certificate/key nélkül is add meg a trusted `tls.public_host` értéket. A proxy minden normál kéréshez állítsa elő az `X-Forwarded-Proto: https` (vagy `Forwarded: proto=https`) metadata-t.

Forwarding header csak trusted proxytól fogadható el.

## Parser hardening

Tiltott többek között: request `Transfer-Encoding`, obs-fold, duplicate Host/Content-Length/Content-Type/Origin/Referer, Upgrade/Expect/Trailer/Proxy-Connection, GET/HEAD body és ismeretlen Connection token.
