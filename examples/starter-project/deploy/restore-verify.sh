#!/bin/sh
set -eu

SERVER=${SERVER:-/usr/local/bin/velran-server}
CLI=${CLI:-/usr/local/bin/velran-cli}
CONFIG=${CONFIG:-/usr/local/etc/velran-restore-test/server.toml}
APP=${APP:-/srv/velran-restore-test/current/main.vrn}
MIGRATIONS=${MIGRATIONS:-/srv/velran-restore-test/current/migrations}
MIGRATION_DB_URL_FILE=${MIGRATION_DB_URL_FILE:-/run/secrets/velran-restore-test/migration-db-url}
DATA_ROOT=${DATA_ROOT:-/srv/velran-restore-test/data}

case "$CONFIG:$APP:$MIGRATIONS:$DATA_ROOT" in
  *'/srv/velran/current/'*|*'/usr/local/etc/velran/server.toml'*|*'/srv/velran/data'*)
    echo 'restore verification refuses the documented production paths' >&2
    exit 1
    ;;
esac

[ -f "$CONFIG" ] || { echo "restore config missing: $CONFIG" >&2; exit 1; }
[ -f "$APP" ] || { echo "restored app missing: $APP" >&2; exit 1; }
[ -d "$MIGRATIONS" ] || { echo "restored migrations missing: $MIGRATIONS" >&2; exit 1; }
[ -d "$DATA_ROOT" ] || { echo "restored data root missing: $DATA_ROOT" >&2; exit 1; }
[ -f "$MIGRATION_DB_URL_FILE" ] || { echo "restore-test migration DB URL file missing: $MIGRATION_DB_URL_FILE" >&2; exit 1; }

"$SERVER" --config "$CONFIG" --app "$APP" --check-config
"$CLI" check "$APP"
"$CLI" migrate verify --dir "$MIGRATIONS" --db-url-file "$MIGRATION_DB_URL_FILE"

printf '%s\n' 'Velran restore verification passed (read-only checks only).'
printf '%s\n' 'Next: start on an isolated listener, then test /health/live, /health/ready and application smoke cases.'
