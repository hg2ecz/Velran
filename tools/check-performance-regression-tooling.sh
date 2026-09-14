#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
P="$ROOT/tools/performance-regression.sh"
fail(){ echo "performance regression tooling: FAIL: $*" >&2; exit 1; }
[ -x "$P" ] || fail 'performance regression runner missing/not executable'
grep -Fq "'/fft4096-x10000'" "$P" || fail 'FFT benchmark endpoint missing'
grep -Fq "'/typed-data-fastpaths'" "$P" || fail 'typed data benchmark endpoint missing'
grep -Fq 'correctness window passed: true' "$P" || fail 'FFT correctness marker is not enforced'
grep -Fq 'MAX_REGRESSION_PERCENT' "$P" || fail 'relative regression threshold missing'
grep -Fq 'sort -n' "$P" || fail 'median sample aggregation missing'
grep -Fq 'schema_version=1' "$P" || fail 'versioned baseline format missing'
echo 'performance regression tooling: PASS'
