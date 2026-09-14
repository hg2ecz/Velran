#!/bin/sh
set -eu
usage(){ echo "usage: $0 record|check <baseline-file>" >&2; exit 2; }
[ "$#" -eq 2 ] || usage
MODE=$1
BASELINE=$2
case "$MODE" in record|check) ;; *) usage ;; esac
BASE_URL=${BASE_URL:-http://127.0.0.1:8080}
WARMUPS=${WARMUPS:-2}
SAMPLES=${SAMPLES:-5}
MAX_REGRESSION_PERCENT=${MAX_REGRESSION_PERCENT:-10}
CURL=${CURL:-curl}
CURL_FLAGS=${CURL_FLAGS:--fsS}
case "$WARMUPS:$SAMPLES:$MAX_REGRESSION_PERCENT" in
  *[!0-9:]*|'') echo 'WARMUPS, SAMPLES and MAX_REGRESSION_PERCENT must be non-negative integers' >&2; exit 2 ;;
esac
[ "$SAMPLES" -gt 0 ] || { echo 'SAMPLES must be > 0' >&2; exit 2; }

fetch(){
  # shellcheck disable=SC2086
  $CURL $CURL_FLAGS "$BASE_URL$1"
}
extract(){
  label=$1
  awk -v label="$label" 'index($0,label){ s=$0; sub(".*" label "[[:space:]]*", "", s); sub("[[:space:]]*ns.*", "", s); gsub(/[^0-9]/,"",s); if(s!=""){print s; exit} }'
}
median_samples(){
  endpoint=$1
  label=$2
  correctness=$3
  i=0
  while [ "$i" -lt "$WARMUPS" ]; do fetch "$endpoint" >/dev/null; i=$((i+1)); done
  tmp=$(mktemp "${TMPDIR:-/tmp}/velran-perf.XXXXXX")
  trap 'rm -f "$tmp"' EXIT HUP INT TERM
  i=0
  while [ "$i" -lt "$SAMPLES" ]; do
    body=$(fetch "$endpoint")
    if [ -n "$correctness" ]; then
      printf '%s\n' "$body" | grep -Fq "$correctness" || { echo "correctness marker missing for $endpoint" >&2; exit 1; }
    fi
    value=$(printf '%s\n' "$body" | extract "$label")
    [ -n "$value" ] || { echo "benchmark value missing for $endpoint ($label)" >&2; exit 1; }
    printf '%s\n' "$value" >> "$tmp"
    i=$((i+1))
  done
  sort -n "$tmp" | awk 'NR==int((n+1)/2){print; exit}' n="$SAMPLES"
  rm -f "$tmp"
  trap - EXIT HUP INT TERM
}

fft=$(median_samples '/fft4096-x10000' 'Average FFT elapsed:' 'correctness window passed: true')
typed=$(median_samples '/typed-data-fastpaths' 'Average iteration:' 'Checksum:')

if [ "$MODE" = record ]; then
  umask 077
  tmp="${BASELINE}.tmp.$$"
  {
    echo 'schema_version=1'
    echo "fft4096_x10000_average_ns=$fft"
    echo "typed_data_fastpaths_average_ns=$typed"
    echo "samples=$SAMPLES"
    echo "max_regression_percent=$MAX_REGRESSION_PERCENT"
  } > "$tmp"
  mv "$tmp" "$BASELINE"
  echo "performance baseline recorded: fft=$fft ns typed=$typed ns -> $BASELINE"
  exit 0
fi

[ -f "$BASELINE" ] || { echo "baseline file missing: $BASELINE" >&2; exit 1; }
# shellcheck disable=SC1090
. "$BASELINE"
[ "${schema_version:-}" = 1 ] || { echo 'unsupported/missing performance baseline schema' >&2; exit 1; }
old_fft=${fft4096_x10000_average_ns:-}
old_typed=${typed_data_fastpaths_average_ns:-}
case "$old_fft:$old_typed" in *[!0-9:]*|:*) echo 'invalid benchmark baseline values' >&2; exit 1;; esac
limit_fft=$((old_fft * (100 + MAX_REGRESSION_PERCENT)))
limit_typed=$((old_typed * (100 + MAX_REGRESSION_PERCENT)))
if [ $((fft * 100)) -gt "$limit_fft" ]; then
  echo "PERF REGRESSION: FFT median $fft ns > baseline $old_fft ns + ${MAX_REGRESSION_PERCENT}%" >&2
  exit 1
fi
if [ $((typed * 100)) -gt "$limit_typed" ]; then
  echo "PERF REGRESSION: typed-data median $typed ns > baseline $old_typed ns + ${MAX_REGRESSION_PERCENT}%" >&2
  exit 1
fi
echo "performance regression gate: PASS (fft=$fft/$old_fft ns typed=$typed/$old_typed ns threshold=${MAX_REGRESSION_PERCENT}%)"
