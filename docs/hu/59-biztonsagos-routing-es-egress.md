<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Biztonságos routing és outbound egress

A Velran a navigációt és a külső hálózati hozzáférést capabilityként kezeli, nem tetszőleges stringként.

## Típusos redirect

Az action deklarált GET route-ra irányít át:

```vrn
return Ok(redirect(AccountView(user.id)));
```

A compiler feloldja a route-ot, ellenőrzi, hogy GET route-e, valamint ellenőrzi az argumentumok számát és típusát. Az AST-ba `RouteCall` kerül, a lokális URL-t pedig a runtime építi fel a route deklarációból. A nyers string redirect cél tiltott (`SEC-A01-011`).

Így az open redirect kikerül a normál nyelvi felületről, és a fejlesztőnek a path/query stringeket sem kell kézzel összeraknia.

## Lokális URL runtime határ

A runtime opaque `LocalUrl` értékkel reprezentálja a redirect célt. `Redirect` nem hozható létre tetszőleges `String`-ből: előbb teljesülnie kell a lokális útvonal invariánsnak. Ez a trusted Rust adaptereket is ugyanazon fail-closed határon tartja.

## Nincs ambient outbound HTTP jogosultság

A Velran forrásban továbbra sincs korlátlan `http.get(String)` primitív. A külső hálózat trusted integration boundary marad. A Rust integration API capability-flow-t használ:

1. `OutboundHttpsClient::capability("payments")` felold egy névvel azonosított egress policyt.
2. `capability.endpoint("api.example.com", 443)` a hostot és portot a policyhez ellenőrzi.
3. `HttpsPath::new("/v1/charges")` gyökeres, header-safe request pathot hoz létre.
4. `client.post_json(&endpoint, &path, ...)` végrehajtja a DNS/CIDR/peer/TLS/méret/timeout ellenőrzéseket.

A transport request hívás többé nem kap különálló `target`, `host`, `port` és path stringeket. Ez csökkenti a policy megkerülésének és a targetek véletlen összekeverésének lehetőségét, miközben a hétköznapi integration kód rövid marad.

## Nyelvi alapelv

> A lokális navigáció route-identitás. A külső hálózat explicit egress capability. Egyik boundary sem fogad tetszőleges, felhasználó által vezérelhető URL stringet.

A későbbi Velran-szintű integration szintaxisnak ugyanezekre a capabilitykre kell fordulnia, nem általános URL-fetch primitívre.
