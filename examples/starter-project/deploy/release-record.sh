#!/bin/sh
set -eu

APP_ROOT=${APP_ROOT:-/srv/velran/current}
CONFIG=${CONFIG:-/usr/local/etc/velran/server.toml}
SERVER=${SERVER:-/usr/local/bin/velran-server}

if [ ! -d "$APP_ROOT" ]; then
  echo "release root not found: $APP_ROOT" >&2
  exit 1
fi
if [ ! -f "$CONFIG" ]; then
  echo "config not found: $CONFIG" >&2
  exit 1
fi
if [ ! -f "$SERVER" ]; then
  echo "server binary not found: $SERVER" >&2
  exit 1
fi

hash_tree() {
  root=$1
  shift
  tmp=$(mktemp)
  trap 'rm -f "$tmp"' EXIT HUP INT TERM
  (
    cd "$root"
    find "$@" -type f -print | LC_ALL=C sort | while IFS= read -r file; do
      sha256sum "$file"
    done
  ) > "$tmp"
  sha256sum "$tmp" | awk '{print $1}'
  rm -f "$tmp"
  trap - EXIT HUP INT TERM
}

app_hash=$(hash_tree "$APP_ROOT" . -name '*.vrn')
if [ -d "$APP_ROOT/migrations" ]; then
  migrations_hash=$(hash_tree "$APP_ROOT/migrations" . -name '*.sql')
else
  migrations_hash=none
fi

printf '%s\n' "recorded_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf '%s\n' "server_sha256=$(sha256sum "$SERVER" | awk '{print $1}')"
printf '%s\n' "app_source_sha256=$app_hash"
printf '%s\n' "migrations_sha256=$migrations_hash"
printf '%s\n' "config_sha256=$(sha256sum "$CONFIG" | awk '{print $1}')"

printf '%s\n' '# Add backup artifact hashes separately; do not append secret values.'
