#!/usr/bin/env bash
set -euo pipefail

require() {
  local pattern=$1 file=$2 message=$3
  if ! grep -q "$pattern" "$file"; then
    echo "native-config: $message" >&2
    exit 1
  fi
}

require 'pub(super) native: FileNative' crates/server/src/server_config_file.rs 'FileNative wiring missing'
require 'pub(super) enabled: Option<bool>' crates/server/src/server_config_file.rs 'native.enabled missing'
require 'pub(super) required: Option<bool>' crates/server/src/server_config_file.rs 'native.required missing'
require 'pub(super) rustc: Option<String>' crates/server/src/server_config_file.rs 'native.rustc missing'
require 'pub(super) cache_dir: Option<String>' crates/server/src/server_config_file.rs 'native.cache_dir missing'
require 'native.optimization' crates/server/src/native_config.rs 'optimization mapping missing'
require 'application_runtime::from_config' crates/server/src/native_runtime.rs 'runtime bootstrap wiring missing'
require 'pub struct NativeRuntimeConfig' crates/native-runtime/src/application_runtime/bootstrap.rs 'NativeRuntimeConfig missing'
require 'settings.rustc_path.as_deref()' crates/native-runtime/src/application_runtime/bootstrap.rs 'configured rustc path is not consumed'

bootstrap_compact=$(tr -d '[:space:]' < crates/native-runtime/src/application_runtime/bootstrap.rs)
case "$bootstrap_compact" in
  *'settings.cache_root.clone()'*) ;;
  *) echo 'native-config: configured cache_root is not consumed' >&2; exit 1 ;;
esac

require 'current.native_config' crates/server/src/source_reload_candidate.rs 'reload candidate native config propagation missing'
require 'target-feature=' crates/native-build/src/rustc_backend/command.rs 'target feature propagation missing'
require '^\[native\]$' config/server.toml.sample 'sample [native] section missing'
require '^enabled = true$' config/server.toml.sample 'sample native enabled=true missing'
require '^required = true$' config/server.toml.sample 'sample native required=true missing'
require '^optimization = "release"$' config/server.toml.sample 'sample native optimization=release missing'
echo 'native server configuration verification passed'
