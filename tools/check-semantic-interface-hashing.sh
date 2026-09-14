#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
F="$ROOT/crates/executable-ir/src/shard_planner/fingerprint.rs"
P="$ROOT/crates/executable-ir/src/shard_planner/planning.rs"
I="$ROOT/crates/native-build/src/incremental_build/mod.rs"
B="$ROOT/crates/native-build/src/build_cache/mod.rs"
A="$ROOT/crates/native-build/src/artifact_format/mod.rs"

grep -q 'interface_sha256(handler' "$F"
grep -q 'implementation_sha256(handler' "$F"
grep -q 'Formatting/comments and private statement bodies cannot change' "$F"
grep -q 'interface_sha256 = fingerprint::shard_sha256' "$P"
grep -q 'implementation_sha256 = fingerprint::shard_sha256' "$P"
grep -q 'interface_changed_shards' "$I"
grep -q 'implementation_changed_shards' "$I"
grep -q 'velran-native-cache-v4-runtime-contract' "$B"
grep -q 'field(&mut bytes, "interface", shard.interface_sha256())' "$B"
grep -q 'field(&mut bytes, "implementation", shard.implementation_sha256())' "$B"
grep -q 'pub const ARTIFACT_MANIFEST_VERSION: u16 = 4' "$A"
grep -q 'pub interface_sha256: String' "$A"
grep -q 'pub implementation_sha256: String' "$A"
echo 'semantic interface/implementation hashing: PASS'
