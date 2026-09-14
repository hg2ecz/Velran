#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
fail(){ echo "provider-features: $*" >&2; exit 1; }
grep -Fq 'default = ["db-sqlite", "db-postgres", "db-mysql", "redis"]' crates/data/Cargo.toml || fail 'data default provider feature set missing'
grep -Fq 'db-postgres = ["sqlx/postgres"]' crates/data/Cargo.toml || fail 'PostgreSQL driver is not feature-gated'
grep -Fq 'db-mysql = ["sqlx/mysql"]' crates/data/Cargo.toml || fail 'MySQL driver is not feature-gated'
grep -Fq 'redis = ["dep:redis"]' crates/data/Cargo.toml || fail 'Redis dependency is not optional'
grep -Fq 'optional = true' crates/data/Cargo.toml || fail 'optional provider dependency missing'
grep -Fq 'ldap = ["dep:ldap3"]' crates/auth/Cargo.toml || fail 'LDAP dependency is not optional'
grep -Fq 'default-features = false' crates/server/Cargo.toml || fail 'server must control provider features explicitly'
grep -Fq 'db-postgres = ["data/db-postgres"]' crates/server/Cargo.toml || fail 'server PostgreSQL feature propagation missing'
grep -Fq 'ldap = ["auth/ldap"]' crates/server/Cargo.toml || fail 'server LDAP feature propagation missing'
printf '%s\n' 'provider feature verification passed'
