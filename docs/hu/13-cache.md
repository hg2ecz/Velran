<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 13. Public page cache

A public cache célja, hogy nyilvános, userfüggetlen GET oldalaknál ne fusson le minden kérésre ugyanaz a DB/query/render munka.

## Deklaráció

```text
route frontPage GET "/" public cache public ttl 60 => frontPage;
```

A fejlesztő csak a cache-elési szándékot és TTL-t deklarálja. A compiler ellenőrzi, hogy a route valóban public GET legyen, és a page ne függjön request-specifikus értéktől, például:

```text
csrfToken
authPrincipal
authMfaVerified
```

Ilyen függésnél compile error történik.

## Cache key

A kulcsot a runtime készíti canonical, hash-elt formában a következőkből:

```text
route name
route generation
path
sorted typed query input
representation (HTML/JSON)
```

A program nem írhat saját Redis/cache key-t.

## Backend

Productionban Redis használatos külön `velran-cache` namespace-szel. Ha cached route van, de nincs Redis, a szerver alapból nem indul el.

Developmenthez explicit:

```bash
--allow-memory-cache
```

A memória cache bounded. Operator limitek:

```text
--cache-max-ttl-secs 3600
--cache-max-entries 10000
--cache-max-bytes 67108864
```

A route TTL nem lépheti túl az operator maximumot.

## Invalidation

State-changing route explicit invalidálhat cache-elt route-ot:

```text
route publish POST "/publish"
    public invalidate cache frontPage article
    => publish;
```

Az invalidation nem Redis `SCAN`/delete-listával működik. Route-generation nő; az új requestek automatikusan új cache namespace-t kapnak. Ez O(1) művelet és cluster-safe Redis esetén.

## Stampede védelem

Ugyanazon instance-en ugyanarra a cache keyre egyszerre csak egy request építi újra az oldalt. A többiek megvárják, majd újra cache-t olvasnak.

## HTTP cache

Sikeres cache-elt response:

```text
Cache-Control: public, max-age=<ttl>
```

Ez azt is jelenti, hogy reverse proxy/CDN cache használható. Ezért csak compiler által public-cache-safe route kaphat ilyen headert.

## Tudatos korlátok

M28-ban nincs:

- private/user cache;
- arbitrary cache key;
- query-level cache;
- tag-based typed invalidation;
- stale-while-revalidate;
- distributed single-flight lease.

Ezek később ugyanarra az infrastruktúrára építhetők.
