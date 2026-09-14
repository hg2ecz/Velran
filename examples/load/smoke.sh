#!/usr/bin/env sh
set -eu
BASE_URL="${BASE_URL:-https://127.0.0.1:8443}"
REQUESTS="${REQUESTS:-200}"
CONCURRENCY="${CONCURRENCY:-20}"

if command -v hey >/dev/null 2>&1; then
  hey -n "$REQUESTS" -c "$CONCURRENCY" "$BASE_URL/"
elif command -v ab >/dev/null 2>&1; then
  ab -n "$REQUESTS" -c "$CONCURRENCY" "$BASE_URL/"
else
  echo "install hey or ab for load smoke testing" >&2
  exit 2
fi
