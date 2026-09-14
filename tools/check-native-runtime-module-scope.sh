#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# Local types of consolidated former crates must resolve through their module, not crate root.
if grep -R -nE 'use crate::(DispatchError|ActivationError|HandlerBinding|RefreshReport|RuntimeError|ApplicationRuntime|BootstrapError)|use crate::\{[^}]*\b(DispatchError|ActivationError|HandlerBinding|RefreshReport|RuntimeError|ApplicationRuntime|BootstrapError)\b' "$root/crates/native-runtime/src/runtime_generation" "$root/crates/native-runtime/src/application_runtime"; then
  echo "native-runtime module-scope verification failed" >&2
  exit 1
fi
echo "native-runtime module-scope verification passed"
