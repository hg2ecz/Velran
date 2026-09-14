<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 9. Observability

A Velran M23 két production-facing observability felületet ad:

- strukturált JSON request/security log;
- Prometheus-kompatibilis metrics listener.

## Structured request log

Alapból engedélyezett. Minden sikeresen parse-olt application/static/health request kap szerver-generált request ID-t:

```http
X-Request-Id: velran-...
```

Példa log:

```json
{"event":"http_request","request_id":"velran-...","method":"GET","route":"products","status":200,"duration_ms":7,"client_ip":"203.0.113.5","bytes_in":0,"bytes_out":812}
```

Trust-boundary tiltásnál külön `security_audit` esemény keletkezik ugyanazzal a request ID-val. A V1 kategorizálja többek között a hibás/untrusted forwarding metadata (`400`, `proxy/invalid_forwarding`), auth (`401`), CSRF/CORS/origin és más policy deny (`403`), Host mismatch (`421`), HTTPS-required (`426`) és rate-limit (`429`) eseteket.

A kliens `X-Request-Id` headerét a runtime nem tekinti trusted correlation ID-nak: az azonosítót mindig a szerver generálja.

Kikapcsolás:

```bash
--no-access-log
```

## Metrics listener

A metrics nem az application route namespace része. Külön listen socketen indul:

```bash
--metrics-listen 127.0.0.1:9090
```

Scrape:

```bash
curl http://127.0.0.1:9090/metrics
```

A listener csak `/metrics` GET/HEAD requestet szolgál ki, session/auth/application handler nélkül.

Nem-loopback bind explicit escape hatch:

```bash
--metrics-listen 10.20.0.15:9090 --allow-public-metrics
```

Ezt csak hálózati ACL/reverse proxy protection mellett használd.

## Fő metrikák

```text
velran_requests_total
velran_responses_5xx_total
velran_auth_failures_total
velran_csrf_failures_total
velran_policy_denials_total
velran_request_timeouts_total
velran_runtime_budget_exceeded_total
velran_rate_limit_denials_total
velran_readiness_failures_total
velran_request_bytes_in_total
velran_response_bytes_out_total
velran_active_connections
velran_request_duration_ms_bucket
velran_route_requests_total{route="..."}
velran_route_5xx_total{route="..."}
velran_route_duration_ms_sum{route="..."}
```

## Cardinality szabály

Prometheus `route` label kizárólag compiler-owned route név vagy fix reserved érték lehet:

```text
__health__
__static__
__media__
__unmatched__
```

Raw URL, path paraméter, IP, username, tenant, session ID vagy request ID **soha nem metric label**.

Ez fontos DoS- és memória-invariáns.

## Mit nem logolunk

Tilos logolni:

```text
Authorization
Cookie/session ID
CSRF token
TOTP secret
DB/Redis/LDAP credential
secret file content
request body
```

A client IP az access log része lehet; adatvédelmi policy szerint a teljes access log kikapcsolható.

## OpenTelemetry

M23 nem húz be OpenTelemetry SDK/exporter dependency-t. A stabil request ID + structured event + Prometheus metric surface későbbi OTel adapter alapja; exporter külön milestone lehet.

## Log classes, audit and SIGHUP

Productionban három külön logikai naplóosztály van:

```toml
[logging]
server_file = "/var/log/velran/server.log"
access_file = "/var/log/velran/access.log"
audit_file = "/var/log/velran/audit.log"
stderr = true
```

### 1. Error/system — `server.log`

Startup/shutdown, dependency és technikai hibák. Az új structured események UTC RFC3339 timestampet, `schema_version`, `level`, `event` és `component` mezőt kapnak.

### 2. Normál kiszolgálás — `access.log`

Minden request structured rekordja: timestamp, request ID, method, route, status, latency, effective client IP és byte-számok. Request body, Cookie, Authorization és credential nincs benne.

### 3. User/security activity — `audit.log`

Automatikusan ide kerül többek között login/logout, auth/MFA failure, policy deny, rate-limit deny, resource-profile startup audit és minden autentikált state-changing application POST/action. Az action a compiler-owned route név, ezért route-neveket érdemes üzletileg beszédesre választani (`articlePublish`, `invoiceApprove`). Form/JSON payload nincs auditálva.

Az `access_log = false` csak az access logot kapcsolja ki; auditot nem.

A három JSON schema jelenleg `schema_version = 1`, az időbélyeg UTC RFC3339 milliszekundum pontossággal készül. Ez támogatja az események időrendi rekonstrukcióját.

### Logrotate / SIGHUP

A **SIGHUP kizárólag logfájl-reopen**. Config reload nincs. Rename után az eredeti path hiányzó fájlja újralétrejön. Reopen-hiba esetén a régi descriptorok maradnak.

A file logging bounded writer queue-t használ; queue/file hiba stderr fallbacket és `velran_log_fallback_total` metrikát okoz.

### Retention és audit-védelem

Retention nem hardcoded: szervezeti, jogi és kockázati policy határozza meg. Az audit trailt productionban érdemes külön központi, hozzáférés-védett/append-only SIEM vagy log archive rendszerbe továbbítani; a service által írható helyi `audit.log` önmagában nem tekintendő erős tamper-evident archívumnak.
