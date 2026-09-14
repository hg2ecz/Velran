<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 7. Lifecycle, health és graceful shutdown

Az M21 beépített liveness/readiness endpointokat és kontrollált leállást ad a szerverhez. Ezek **szerver endpointok**, nem `.vrn` route-ok, és nem hoznak létre sessiont.

## Liveness

Alapértelmezés:

```text
GET /health/live
HEAD /health/live
```

Válasz:

```json
{"status":"live"}
```

A liveness csak azt jelzi, hogy a szerver event loop él. Nem hív DB-t, Redist vagy LDAP-ot.

## Readiness

```text
GET /health/ready
HEAD /health/ready
```

Ha minden konfigurált runtime dependency elérhető:

```json
{"status":"ready"}
```

DB vagy Redis hiba/timeout esetén:

```http
HTTP/1.1 503 Service Unavailable
```

```json
{"status":"not_ready"}
```

A readiness jelenleg a konfigurált DB-t és Redis auth/session store-t ellenőrzi. LDAP nincs probe-onként ellenőrizve.

Mindkét health válasz `Cache-Control: no-store`. POST és más method `405`.

## Konfiguráció

Productionban a lifecycle policy a trusted TOML config része:

```toml
[lifecycle]
health_live_path = "/health/live"
health_ready_path = "/health/ready"
health_dependency_timeout_ms = 1000
shutdown_grace_ms = 30000
```

A megfelelő CLI flag-ek célzott development/ops override-ként továbbra is használhatók, de ne ezekből épüljön a production policy.

A két pathnak különböznie kell. Ha application route vagy static namespace ütközik a health pathszal, a szerver fail-closed módon nem indul. Dinamikus route (például `/health/:name`) is ütközésnek számít.

## Graceful shutdown

SIGTERM vagy SIGINT esetén:

```text
1. új connection accept leáll
2. redirect listener leáll
3. aktív connection taskok drain
4. shutdown_grace_ms lejártakor maradék task abort
5. process kilép
```

Ez Kubernetes/systemd deploymentnél megakadályozza, hogy a normál rolling restart azonnal elvágja az aktív requesteket. A külső load balancer/orchestrator termination grace periodja legyen **nagyobb**, mint a Velran `lifecycle.shutdown_grace_ms` értéke.
