#!/bin/sh
set -eu

SERVER=${SERVER:-/usr/local/bin/velran-server}
CLI=${CLI:-/usr/local/bin/velran-cli}
CONFIG=${CONFIG:-/usr/local/etc/velran/server.toml}
APP=${APP:-/srv/velran/current/main.vrn}
MIGRATIONS=${MIGRATIONS:-/srv/velran/current/migrations}
MIGRATION_DB_URL_FILE=${MIGRATION_DB_URL_FILE:-/run/secrets/velran/migration-db-url}

"$SERVER" --config "$CONFIG" --app "$APP" --check-config
"$CLI" check "$APP"
"$CLI" migrate verify --dir "$MIGRATIONS" --db-url-file "$MIGRATION_DB_URL_FILE"

echo "Velran preflight passed; backup/restore readiness must be confirmed separately before schema mutation."
