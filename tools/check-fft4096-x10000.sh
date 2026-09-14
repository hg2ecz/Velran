#!/usr/bin/env bash
set -euo pipefail

src='examples/fft4096-x10000/main.vrn'
manifest='tests/manifests/example-entrypoints.txt'

test -f "$src"
grep -Fq 'let runs = 10000;' "$src"
grep -Fq 'while run < runs {' "$src"
grep -Fq 'let started = std::time::Instant::now();' "$src"
grep -Fq 'let fft_ns = started.elapsed().as_nanos();' "$src"
grep -Fq 'total_fft_ns += fft_ns;' "$src"
grep -Fq 'let average_fft_ns = total_fft_ns / runs;' "$src"
grep -Fq 'real[i] = base_real[i];' "$src"
grep -Fq 'imag[i] = base_imag[i];' "$src"
grep -Fq 'real[0] = base_real[0] + run as f32 * 0.0000001f32;' "$src"
grep -Fq 'checksum += real[64];' "$src"
grep -Fq 'checksum += imag[64];' "$src"
grep -Fq 'checksum += real[256];' "$src"
grep -Fq 'checksum += imag[256];' "$src"
grep -Fq 'Anti-optimization checksum: {{ checksum }}' "$src"
grep -Fq 'correctness window passed: {{ correct }}' "$src"
grep -Fxq 'examples/fft4096-x10000/main.vrn' "$manifest"

# Preserve the same core butterfly as the single-run benchmark.
for needle in \
    'let even = i + j;' \
    'let odd = even + half;' \
    'let vr = real[odd] * wr - imag[odd] * wi;' \
    'real[even] = ur + vr;' \
    'real[odd] = ur - vr;'
do
    grep -Fq "$needle" "$src"
done

echo 'fft4096 x10000 benchmark gate: PASS'
