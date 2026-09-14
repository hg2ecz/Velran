<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 8. Route rate limiting

A Velran route egy **névvel ellátott, trusted configban definiált policy-t** kérhet:

```text
route api GET "/api/status" public rate publicApi => api;
```

A route nem adhat meg saját limit-számokat.

## Policy config

```toml
[policy.publicApi]
limit = 120
window_secs = 60
scope = "ip_route"
```

Indítás productionban:

```bash
velran-server \
  --app /srv/myapp/main.vrn \
  --rate-limits-file /usr/local/etc/velran/rate-limits.toml \
  --redis-url-file /run/secrets/redis-url \
  ...
```

Ha van route rate policy, productionban Redis szükséges. A memória backend csak explicit development escape hatch:

```bash
--allow-memory-rate-limit
```

## Scope-ok

```text
ip          kliens IP
route       teljes route közös bucket
ip_route    route + kliens IP
user        authenticated principal
user_route  route + authenticated principal
```

`user`/`user_route` public route-on startup error.

A trusted proxy szabályok után számított effective client IP kerül a limiterbe, így csak `--trusted-proxy-cidr` által engedélyezett proxy befolyásolhatja a kliens IP-t.

## Válasz limit esetén

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 60
```

JSON API-n:

```json
{"error":"rate_limited"}
```

Redis hiba esetén fail-closed:

```text
503 rate_limiter_unavailable
```

## Miért nincs tenant scope v0.1-ben?

A runtime-ban még nincs első osztályú, trusted tenant identity. User inputból érkező `tenantId` nem lehet security authority, ezért tenant-scoped limiter csak egy későbbi typed tenant capability után kerülhet be.

## Modell

A limiter fixed-window bucketet használ. Redisben minden instance ugyanazt a bucket-kulcsot növeli atomikusan. A subjectet stabil hash-re alakítjuk, így username/IP nincs nyersen a Redis key-ben.
